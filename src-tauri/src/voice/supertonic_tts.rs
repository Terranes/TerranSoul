//! On-device Supertonic TTS engine (ONNX Runtime).
//!
//! This module is compiled only when the `tts-supertonic` feature is enabled
//! (default on desktop builds). It adapts the upstream Rust example
//! ([`supertone-inc/supertonic/rust/src/helper.rs`](https://github.com/supertone-inc/supertonic),
//! MIT) into a `TtsEngine` implementation that loads the four ONNX sessions,
//! the unicode indexer and one or more voice-style JSON files from disk
//! and runs the four-stage synthesis pipeline:
//! `duration_predictor → text_encoder → vector_estimator (iterative)
//! → vocoder`.
//!
//! ## License & attribution
//!
//! Sample-code adaptation is MIT (matches upstream). Model weights are
//! OpenRAIL-M and are not bundled — they are downloaded on first use via
//! [`super::supertonic_download`] only after the user accepts the consent
//! dialog shipped in stage 1c. See [`docs/licensing-audit.md`](../../../docs/licensing-audit.md)
//! 🟡 Conditional clearance section.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use hound::{SampleFormat, WavSpec, WavWriter};
use ndarray::{Array, Array3};
use ort::session::Session;
use ort::value::Value;
use rand_distr::{Distribution, Normal};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use unicode_normalization::UnicodeNormalization;

use super::supertonic_manifest::VOICES;
use super::{SynthesisResult, TtsEngine};

/// Languages the v2 model supports. Wrapping a text in `<lang>...</lang>`
/// tags is required by the model (see upstream `preprocess_text`).
pub const AVAILABLE_LANGS: &[&str] = &["en", "ko", "es", "pt", "fr"];

const DEFAULT_LANG: &str = "en";
const DEFAULT_VOICE: &str = "F1";
const DEFAULT_TOTAL_STEP: usize = 5;
const DEFAULT_SPEED: f32 = 1.05;
const DEFAULT_SILENCE_BETWEEN_CHUNKS_SECS: f32 = 0.2;
const MAX_CHUNK_LEN_DEFAULT: usize = 300;
const MAX_CHUNK_LEN_KO: usize = 120;

// ── Config / style structures ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AEConfig {
    sample_rate: i32,
    base_chunk_size: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TTLConfig {
    chunk_compress_factor: i32,
    latent_dim: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PipelineConfig {
    ae: AEConfig,
    ttl: TTLConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VoiceStyleData {
    style_ttl: StyleComponent,
    style_dp: StyleComponent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StyleComponent {
    data: Vec<Vec<Vec<f32>>>,
    dims: Vec<usize>,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    dtype: String,
}

#[derive(Debug, Clone)]
struct Style {
    ttl: Array3<f32>,
    dp: Array3<f32>,
}

// ── Unicode-indexer-based text processor ─────────────────────────────────────

struct UnicodeProcessor {
    indexer: Vec<i64>,
}

impl UnicodeProcessor {
    fn load(path: &Path) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("open unicode indexer: {e}"))?;
        let indexer: Vec<i64> = serde_json::from_reader(BufReader::new(file))
            .map_err(|e| format!("parse unicode indexer: {e}"))?;
        Ok(Self { indexer })
    }

    fn encode(&self, text: &str, lang: &str) -> Result<(Array<i64, ndarray::Ix2>, Array3<f32>), String> {
        let processed = preprocess_text(text, lang)?;
        let chars: Vec<char> = processed.chars().collect();
        let len = chars.len();
        let max_len = len.max(1);
        let mut row = vec![0i64; max_len];
        for (j, c) in chars.into_iter().enumerate() {
            let val = c as usize;
            row[j] = if val < self.indexer.len() {
                self.indexer[val]
            } else {
                -1
            };
        }
        let text_ids = Array::from_shape_vec((1, max_len), row)
            .map_err(|e| format!("shape text_ids: {e}"))?;
        let text_mask = length_to_mask(&[len], max_len);
        Ok((text_ids, text_mask))
    }
}

fn length_to_mask(lengths: &[usize], max_len: usize) -> Array3<f32> {
    let bsz = lengths.len();
    let mut mask = Array3::<f32>::zeros((bsz, 1, max_len));
    for (i, &len) in lengths.iter().enumerate() {
        for j in 0..len.min(max_len) {
            mask[[i, 0, j]] = 1.0;
        }
    }
    mask
}

fn preprocess_text(text: &str, lang: &str) -> Result<String, String> {
    if !AVAILABLE_LANGS.contains(&lang) {
        return Err(format!("Unsupported Supertonic language: {lang}"));
    }
    let mut text: String = text.nfkd().collect();
    // Remove a broad swath of emoji / pictographic ranges. Pattern lifted from
    // the upstream MIT example.
    let emoji_re = Regex::new(
        r"[\x{1F600}-\x{1F64F}\x{1F300}-\x{1F5FF}\x{1F680}-\x{1F6FF}\x{1F700}-\x{1F77F}\x{1F780}-\x{1F7FF}\x{1F800}-\x{1F8FF}\x{1F900}-\x{1F9FF}\x{1FA00}-\x{1FA6F}\x{1FA70}-\x{1FAFF}\x{2600}-\x{26FF}\x{2700}-\x{27BF}\x{1F1E6}-\x{1F1FF}]+",
    )
    .map_err(|e| format!("emoji regex: {e}"))?;
    text = emoji_re.replace_all(&text, "").to_string();

    let replacements = [
        ("–", "-"),
        ("‑", "-"),
        ("—", "-"),
        ("_", " "),
        ("\u{201C}", "\""),
        ("\u{201D}", "\""),
        ("\u{2018}", "'"),
        ("\u{2019}", "'"),
        ("´", "'"),
        ("`", "'"),
        ("[", " "),
        ("]", " "),
        ("|", " "),
        ("/", " "),
        ("#", " "),
        ("→", " "),
        ("←", " "),
    ];
    for (from, to) in replacements {
        text = text.replace(from, to);
    }
    for sym in ["♥", "☆", "♡", "©", "\\"] {
        text = text.replace(sym, "");
    }
    for (from, to) in [("@", " at "), ("e.g.,", "for example, "), ("i.e.,", "that is, ")] {
        text = text.replace(from, to);
    }

    // Strip whitespace before common punctuation.
    for pat in [r" ,", r" \.", r" !", r" \?", r" ;", r" :", r" '"] {
        let re = Regex::new(pat).map_err(|e| format!("punct regex: {e}"))?;
        let target = pat.trim_start().trim_start_matches('\\');
        text = re.replace_all(&text, target).to_string();
    }
    while text.contains("\"\"") {
        text = text.replace("\"\"", "\"");
    }
    while text.contains("''") {
        text = text.replace("''", "'");
    }
    while text.contains("``") {
        text = text.replace("``", "`");
    }
    let ws = Regex::new(r"\s+").map_err(|e| format!("ws regex: {e}"))?;
    text = ws.replace_all(&text, " ").to_string();
    text = text.trim().to_string();
    if !text.is_empty() {
        let ends_with_punct =
            Regex::new(r#"[.!?;:,'"\u{201C}\u{201D}\u{2018}\u{2019})\]}…。」』】〉》›»]$"#)
                .map_err(|e| format!("end-punct regex: {e}"))?;
        if !ends_with_punct.is_match(&text) {
            text.push('.');
        }
    }
    text = format!("<{lang}>{text}</{lang}>");
    Ok(text)
}

// ── Sentence chunking (port of upstream `chunk_text`). ───────────────────────

const ABBREVIATIONS: &[&str] = &[
    "Dr.", "Mr.", "Mrs.", "Ms.", "Prof.", "Sr.", "Jr.", "St.", "Ave.", "Rd.", "Blvd.", "Dept.",
    "Inc.", "Ltd.", "Co.", "Corp.", "etc.", "vs.", "i.e.", "e.g.", "Ph.D.",
];

fn chunk_text(text: &str, max_len: usize) -> Vec<String> {
    let text = text.trim();
    if text.is_empty() {
        return vec![String::new()];
    }
    let para_re = Regex::new(r"\n\s*\n").unwrap();
    let paragraphs: Vec<&str> = para_re.split(text).collect();
    let mut chunks: Vec<String> = Vec::new();
    for para in paragraphs {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }
        if para.chars().count() <= max_len {
            chunks.push(para.to_string());
            continue;
        }
        let sentences = split_sentences(para);
        let mut current = String::new();
        for sentence in sentences {
            let s = sentence.trim();
            if s.is_empty() {
                continue;
            }
            if s.chars().count() > max_len {
                if !current.is_empty() {
                    chunks.push(current.trim().to_string());
                    current.clear();
                }
                // Fall back to comma splitting.
                let mut acc = String::new();
                for part in s.split(',') {
                    let p = part.trim();
                    if p.is_empty() {
                        continue;
                    }
                    if acc.chars().count() + p.chars().count() + 2 > max_len && !acc.is_empty() {
                        chunks.push(acc.trim().to_string());
                        acc.clear();
                    }
                    if !acc.is_empty() {
                        acc.push_str(", ");
                    }
                    acc.push_str(p);
                }
                if !acc.is_empty() {
                    chunks.push(acc.trim().to_string());
                }
                continue;
            }
            if current.chars().count() + s.chars().count() + 1 > max_len && !current.is_empty() {
                chunks.push(current.trim().to_string());
                current.clear();
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(s);
        }
        if !current.is_empty() {
            chunks.push(current.trim().to_string());
        }
    }
    if chunks.is_empty() {
        vec![String::new()]
    } else {
        chunks
    }
}

fn split_sentences(text: &str) -> Vec<String> {
    let re = Regex::new(r"([.!?])\s+").unwrap();
    let matches: Vec<_> = re.find_iter(text).collect();
    if matches.is_empty() {
        return vec![text.to_string()];
    }
    let mut out = Vec::new();
    let mut last_end = 0usize;
    for m in matches {
        let before = &text[last_end..m.start()];
        let mut is_abbrev = false;
        for ab in ABBREVIATIONS {
            let combined = format!("{}{}", before.trim(), &text[m.start()..m.start() + 1]);
            if combined.ends_with(ab) {
                is_abbrev = true;
                break;
            }
        }
        if !is_abbrev {
            out.push(text[last_end..m.end()].to_string());
            last_end = m.end();
        }
    }
    if last_end < text.len() {
        out.push(text[last_end..].to_string());
    }
    if out.is_empty() {
        vec![text.to_string()]
    } else {
        out
    }
}

// ── ONNX session wrapper ─────────────────────────────────────────────────────

struct InferenceState {
    cfg: PipelineConfig,
    text_processor: UnicodeProcessor,
    dp: Session,
    text_enc: Session,
    vector_est: Session,
    vocoder: Session,
}

impl InferenceState {
    fn load(install_dir: &Path) -> Result<Self, String> {
        let onnx_dir = install_dir.join("onnx");
        let cfg_path = onnx_dir.join("tts.json");
        let cfg_file = File::open(&cfg_path)
            .map_err(|e| format!("open {}: {e}", cfg_path.display()))?;
        let cfg: PipelineConfig = serde_json::from_reader(BufReader::new(cfg_file))
            .map_err(|e| format!("parse tts.json: {e}"))?;

        let text_processor = UnicodeProcessor::load(&onnx_dir.join("unicode_indexer.json"))?;

        let mk_session = |name: &str| -> Result<Session, String> {
            Session::builder()
                .map_err(|e| format!("session builder: {e}"))?
                .commit_from_file(onnx_dir.join(name))
                .map_err(|e| format!("load {name}: {e}"))
        };
        let dp = mk_session("duration_predictor.onnx")?;
        let text_enc = mk_session("text_encoder.onnx")?;
        let vector_est = mk_session("vector_estimator.onnx")?;
        let vocoder = mk_session("vocoder.onnx")?;

        Ok(Self {
            cfg,
            text_processor,
            dp,
            text_enc,
            vector_est,
            vocoder,
        })
    }

    fn sample_rate(&self) -> i32 {
        self.cfg.ae.sample_rate
    }

    fn infer_one(
        &mut self,
        text: &str,
        lang: &str,
        style: &Style,
        total_step: usize,
        speed: f32,
    ) -> Result<(Vec<f32>, f32), String> {
        let (text_ids, text_mask) = self.text_processor.encode(text, lang)?;

        let text_ids_value =
            Value::from_array(text_ids.clone()).map_err(|e| format!("text_ids value: {e}"))?;
        let text_mask_value =
            Value::from_array(text_mask.clone()).map_err(|e| format!("text_mask value: {e}"))?;
        let style_dp_value =
            Value::from_array(style.dp.clone()).map_err(|e| format!("style_dp value: {e}"))?;

        let dp_outputs = self
            .dp
            .run(ort::inputs! {
                "text_ids" => &text_ids_value,
                "style_dp" => &style_dp_value,
                "text_mask" => &text_mask_value,
            })
            .map_err(|e| format!("duration_predictor run: {e}"))?;

        let (_, duration_data) = dp_outputs["duration"]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("extract duration: {e}"))?;
        let mut duration: Vec<f32> = duration_data.to_vec();
        for d in duration.iter_mut() {
            *d /= speed;
        }

        let style_ttl_value =
            Value::from_array(style.ttl.clone()).map_err(|e| format!("style_ttl value: {e}"))?;
        let text_enc_outputs = self
            .text_enc
            .run(ort::inputs! {
                "text_ids" => &text_ids_value,
                "style_ttl" => &style_ttl_value,
                "text_mask" => &text_mask_value,
            })
            .map_err(|e| format!("text_encoder run: {e}"))?;
        let (text_emb_shape, text_emb_data) = text_enc_outputs["text_emb"]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("extract text_emb: {e}"))?;
        let text_emb = Array3::from_shape_vec(
            (
                text_emb_shape[0] as usize,
                text_emb_shape[1] as usize,
                text_emb_shape[2] as usize,
            ),
            text_emb_data.to_vec(),
        )
        .map_err(|e| format!("reshape text_emb: {e}"))?;

        let (mut xt, latent_mask) = sample_noisy_latent(
            &duration,
            self.cfg.ae.sample_rate,
            self.cfg.ae.base_chunk_size,
            self.cfg.ttl.chunk_compress_factor,
            self.cfg.ttl.latent_dim,
        );

        let bsz = duration.len();
        let total_step_array = Array::from_elem(bsz, total_step as f32);
        for step in 0..total_step {
            let current_step_array = Array::from_elem(bsz, step as f32);
            let xt_value =
                Value::from_array(xt.clone()).map_err(|e| format!("xt value: {e}"))?;
            let text_emb_value = Value::from_array(text_emb.clone())
                .map_err(|e| format!("text_emb value: {e}"))?;
            let latent_mask_value = Value::from_array(latent_mask.clone())
                .map_err(|e| format!("latent_mask value: {e}"))?;
            let text_mask_value2 = Value::from_array(text_mask.clone())
                .map_err(|e| format!("text_mask2 value: {e}"))?;
            let current_step_value = Value::from_array(current_step_array)
                .map_err(|e| format!("current_step value: {e}"))?;
            let total_step_value = Value::from_array(total_step_array.clone())
                .map_err(|e| format!("total_step value: {e}"))?;
            let outputs = self
                .vector_est
                .run(ort::inputs! {
                    "noisy_latent" => &xt_value,
                    "text_emb" => &text_emb_value,
                    "style_ttl" => &style_ttl_value,
                    "latent_mask" => &latent_mask_value,
                    "text_mask" => &text_mask_value2,
                    "current_step" => &current_step_value,
                    "total_step" => &total_step_value,
                })
                .map_err(|e| format!("vector_estimator run (step {step}): {e}"))?;
            let (shape, data) = outputs["denoised_latent"]
                .try_extract_tensor::<f32>()
                .map_err(|e| format!("extract denoised_latent: {e}"))?;
            xt = Array3::from_shape_vec(
                (shape[0] as usize, shape[1] as usize, shape[2] as usize),
                data.to_vec(),
            )
            .map_err(|e| format!("reshape denoised_latent: {e}"))?;
        }

        let final_latent_value =
            Value::from_array(xt).map_err(|e| format!("final latent value: {e}"))?;
        let voc_outputs = self
            .vocoder
            .run(ort::inputs! { "latent" => &final_latent_value })
            .map_err(|e| format!("vocoder run: {e}"))?;
        let (_, wav_data) = voc_outputs["wav_tts"]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("extract wav_tts: {e}"))?;
        let wav: Vec<f32> = wav_data.to_vec();
        let dur_total = duration[0];
        Ok((wav, dur_total))
    }
}

fn sample_noisy_latent(
    duration: &[f32],
    sample_rate: i32,
    base_chunk_size: i32,
    chunk_compress: i32,
    latent_dim: i32,
) -> (Array3<f32>, Array3<f32>) {
    let bsz = duration.len();
    let max_dur = duration.iter().fold(0.0f32, |a, &b| a.max(b));
    let wav_len_max = (max_dur * sample_rate as f32) as usize;
    let wav_lengths: Vec<usize> = duration
        .iter()
        .map(|&d| (d * sample_rate as f32) as usize)
        .collect();
    let chunk_size = (base_chunk_size * chunk_compress) as usize;
    let latent_len = wav_len_max.div_ceil(chunk_size.max(1));
    let latent_dim_val = (latent_dim * chunk_compress) as usize;

    let mut noisy = Array3::<f32>::zeros((bsz, latent_dim_val, latent_len));
    let normal = Normal::new(0.0, 1.0).expect("Normal(0,1) is always valid");
    let mut rng = rand::rng();
    for b in 0..bsz {
        for d in 0..latent_dim_val {
            for t in 0..latent_len {
                noisy[[b, d, t]] = normal.sample(&mut rng);
            }
        }
    }
    let latent_lengths: Vec<usize> = wav_lengths
        .iter()
        .map(|&l| l.div_ceil(chunk_size.max(1)))
        .collect();
    let latent_mask = length_to_mask(&latent_lengths, latent_len);
    for b in 0..bsz {
        for d in 0..latent_dim_val {
            for t in 0..latent_len {
                noisy[[b, d, t]] *= latent_mask[[b, 0, t]];
            }
        }
    }
    (noisy, latent_mask)
}

fn load_voice_style(install_dir: &Path, voice: &str) -> Result<Style, String> {
    let path = install_dir.join("voice_styles").join(format!("{voice}.json"));
    let file = File::open(&path).map_err(|e| format!("open {}: {e}", path.display()))?;
    let data: VoiceStyleData = serde_json::from_reader(BufReader::new(file))
        .map_err(|e| format!("parse voice style {voice}: {e}"))?;
    let ttl_dim1 = *data
        .style_ttl
        .dims
        .get(1)
        .ok_or_else(|| "style_ttl dims missing axis 1".to_string())?;
    let ttl_dim2 = *data
        .style_ttl
        .dims
        .get(2)
        .ok_or_else(|| "style_ttl dims missing axis 2".to_string())?;
    let dp_dim1 = *data
        .style_dp
        .dims
        .get(1)
        .ok_or_else(|| "style_dp dims missing axis 1".to_string())?;
    let dp_dim2 = *data
        .style_dp
        .dims
        .get(2)
        .ok_or_else(|| "style_dp dims missing axis 2".to_string())?;
    let ttl_flat: Vec<f32> = data
        .style_ttl
        .data
        .into_iter()
        .flatten()
        .flatten()
        .collect();
    let dp_flat: Vec<f32> = data
        .style_dp
        .data
        .into_iter()
        .flatten()
        .flatten()
        .collect();
    let ttl =
        Array3::from_shape_vec((1, ttl_dim1, ttl_dim2), ttl_flat)
            .map_err(|e| format!("reshape ttl: {e}"))?;
    let dp = Array3::from_shape_vec((1, dp_dim1, dp_dim2), dp_flat)
        .map_err(|e| format!("reshape dp: {e}"))?;
    Ok(Style { ttl, dp })
}

fn encode_wav(samples: &[f32], sample_rate: i32) -> Result<Vec<u8>, String> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: sample_rate as u32,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut buf: Vec<u8> = Vec::with_capacity(samples.len() * 2 + 44);
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut writer = WavWriter::new(cursor, spec).map_err(|e| format!("wav writer: {e}"))?;
        for &s in samples {
            let clamped = s.clamp(-1.0, 1.0);
            let v = (clamped * 32767.0) as i16;
            writer
                .write_sample(v)
                .map_err(|e| format!("wav write_sample: {e}"))?;
        }
        writer.finalize().map_err(|e| format!("wav finalize: {e}"))?;
    }
    Ok(buf)
}

// ── Public engine type ───────────────────────────────────────────────────────

/// Synthesis options. `Default` returns the recommended values for everyday
/// use (F1 voice, English, 5 denoising steps).
#[derive(Debug, Clone)]
pub struct SupertonicOptions {
    pub voice: String,
    pub lang: String,
    pub total_step: usize,
    pub speed: f32,
}

impl Default for SupertonicOptions {
    fn default() -> Self {
        Self {
            voice: DEFAULT_VOICE.to_string(),
            lang: DEFAULT_LANG.to_string(),
            total_step: DEFAULT_TOTAL_STEP,
            speed: DEFAULT_SPEED,
        }
    }
}

/// On-device Supertonic TTS engine. One instance owns the four ONNX sessions
/// and one preloaded voice style. Cloning a handle shares the underlying
/// sessions via `Arc`.
pub struct SupertonicTts {
    inner: Arc<Mutex<InferenceState>>,
    install_dir: PathBuf,
    style: Style,
    options: SupertonicOptions,
}

impl SupertonicTts {
    /// Load all four ONNX sessions + tokeniser + the default voice style.
    /// Heavy — ~250 MB of weights are mmapped into the process. Run inside
    /// `tokio::task::spawn_blocking` from async contexts.
    pub fn load(install_dir: &Path, options: SupertonicOptions) -> Result<Self, String> {
        if !VOICES.contains(&options.voice.as_str()) {
            return Err(format!("Unknown Supertonic voice: {}", options.voice));
        }
        let state = InferenceState::load(install_dir)?;
        let style = load_voice_style(install_dir, &options.voice)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(state)),
            install_dir: install_dir.to_path_buf(),
            style,
            options,
        })
    }

    pub fn install_dir(&self) -> &Path {
        &self.install_dir
    }

    pub fn options(&self) -> &SupertonicOptions {
        &self.options
    }

    async fn synthesize_to_wav(&self, text: &str) -> Result<Vec<u8>, String> {
        let max_len = if self.options.lang == "ko" {
            MAX_CHUNK_LEN_KO
        } else {
            MAX_CHUNK_LEN_DEFAULT
        };
        let chunks = chunk_text(text, max_len);

        let inner = Arc::clone(&self.inner);
        let style = self.style.clone();
        let lang = self.options.lang.clone();
        let total_step = self.options.total_step;
        let speed = self.options.speed;
        let owned_chunks: Vec<String> = chunks;

        // Heavy synchronous work — run on the blocking pool so the runtime
        // stays responsive for other voice paths.
        let (samples, sample_rate) = tokio::task::spawn_blocking(move || {
            // Acquire the session lock once for the whole utterance — ort
            // Sessions are not Sync-safe for concurrent `.run()` calls.
            let mut guard = inner.blocking_lock();
            let sample_rate = guard.sample_rate();
            let mut wav_cat: Vec<f32> = Vec::new();
            let silence_len =
                (DEFAULT_SILENCE_BETWEEN_CHUNKS_SECS * sample_rate as f32) as usize;
            for (i, chunk) in owned_chunks.iter().enumerate() {
                if chunk.is_empty() {
                    continue;
                }
                let (wav, dur) = guard.infer_one(chunk, &lang, &style, total_step, speed)?;
                let wav_len = (sample_rate as f32 * dur) as usize;
                let slice = &wav[..wav_len.min(wav.len())];
                if i > 0 {
                    wav_cat.extend(std::iter::repeat_n(0.0f32, silence_len));
                }
                wav_cat.extend_from_slice(slice);
            }
            Ok::<_, String>((wav_cat, sample_rate))
        })
        .await
        .map_err(|e| format!("supertonic blocking task: {e}"))??;

        encode_wav(&samples, sample_rate)
    }
}

#[async_trait]
impl TtsEngine for SupertonicTts {
    fn id(&self) -> &str {
        "supertonic"
    }

    fn display_name(&self) -> &str {
        "Supertonic (on-device, neural)"
    }

    async fn synthesize(&self, text: &str) -> Result<SynthesisResult, String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err("Text cannot be empty".to_string());
        }
        let audio = self.synthesize_to_wav(trimmed).await?;
        let sample_rate = {
            let guard = self.inner.lock().await;
            guard.sample_rate() as u32
        };
        Ok(SynthesisResult {
            audio,
            mime_type: "audio/wav".to_string(),
            sample_rate,
        })
    }

    async fn health_check(&self) -> bool {
        // Sessions were loaded in `load()`; if the struct exists, they are
        // ready to synthesize.
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preprocess_text_wraps_in_language_tags() {
        let out = preprocess_text("hello world", "en").unwrap();
        assert!(out.starts_with("<en>"), "got: {out}");
        assert!(out.ends_with("</en>"), "got: {out}");
        assert!(out.contains("hello world"));
    }

    #[test]
    fn preprocess_text_appends_period_when_missing() {
        let out = preprocess_text("hello", "en").unwrap();
        assert!(out.contains("hello."));
    }

    #[test]
    fn preprocess_text_rejects_invalid_lang() {
        assert!(preprocess_text("hello", "zz").is_err());
    }

    #[test]
    fn preprocess_text_strips_emojis_and_normalises_whitespace() {
        let out = preprocess_text("hello 👋  world", "en").unwrap();
        assert!(!out.contains('👋'));
        assert!(!out.contains("  "));
    }

    #[test]
    fn chunk_text_emits_at_least_one_chunk_for_empty_input() {
        assert_eq!(chunk_text("", 100), vec![String::new()]);
    }

    #[test]
    fn chunk_text_keeps_short_paragraphs_whole() {
        let out = chunk_text("Hello world.", 100);
        assert_eq!(out, vec!["Hello world.".to_string()]);
    }

    #[test]
    fn chunk_text_splits_long_paragraphs_on_sentences() {
        let para = "Sentence one is here. Sentence two follows. Sentence three is the last.";
        let out = chunk_text(para, 30);
        assert!(out.len() >= 2, "expected multiple chunks, got {out:?}");
        for c in &out {
            assert!(c.chars().count() <= 60, "chunk too long: {c:?}");
        }
    }

    #[test]
    fn length_to_mask_marks_only_active_positions() {
        let m = length_to_mask(&[3], 5);
        assert_eq!(m[[0, 0, 0]], 1.0);
        assert_eq!(m[[0, 0, 2]], 1.0);
        assert_eq!(m[[0, 0, 3]], 0.0);
        assert_eq!(m[[0, 0, 4]], 0.0);
    }

    #[test]
    fn encode_wav_writes_riff_header() {
        let samples = vec![0.0_f32; 16];
        let bytes = encode_wav(&samples, 16_000).unwrap();
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        // 16 mono i16 samples = 32 bytes of data + 44 byte header.
        assert!(bytes.len() >= 44 + 32);
    }

    #[test]
    fn supertonic_options_default_uses_f1_english() {
        let opts = SupertonicOptions::default();
        assert_eq!(opts.voice, "F1");
        assert_eq!(opts.lang, "en");
        assert_eq!(opts.total_step, 5);
    }

    /// Real-model synthesis test. Gated on `--ignored` and requires the
    /// Supertonic model to be installed under
    /// `<repo>/target-test/supertonic-fixture/` (or the path provided via
    /// `TERRANSOUL_SUPERTONIC_DIR`). The chunk-1b acceptance procedure runs
    /// this with the fixture directory pointing at a real download.
    #[tokio::test]
    #[ignore = "requires real Supertonic model on disk; opt-in via --ignored"]
    async fn integration_synthesizes_short_utterance() {
        let dir = std::env::var("TERRANSOUL_SUPERTONIC_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("target-test/supertonic-fixture"));
        let tts = SupertonicTts::load(&dir, SupertonicOptions::default())
            .expect("Supertonic model must be installed before running this test");
        let result = tts
            .synthesize("Hello from TerranSoul.")
            .await
            .expect("synthesis should succeed");
        assert_eq!(result.mime_type, "audio/wav");
        assert!(result.audio.len() > 1024, "WAV should contain audio samples");
        assert_eq!(&result.audio[0..4], b"RIFF");
    }
}
