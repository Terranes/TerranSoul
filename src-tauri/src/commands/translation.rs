use serde::{Deserialize, Serialize};

/// Common language choices (ISO 639-1) exposed to UI/plugin callers.
/// Translation accepts any syntactically valid BCP-47/ISO language code so
/// worldwide languages and regional variants are not blocked by this list.
pub const SUPPORTED_LANGUAGES: &[(&str, &str)] = &[
    ("aa", "Afar"),
    ("ab", "Abkhazian"),
    ("ae", "Avestan"),
    ("af", "Afrikaans"),
    ("ak", "Akan"),
    ("am", "Amharic"),
    ("an", "Aragonese"),
    ("ar", "Arabic"),
    ("as", "Assamese"),
    ("av", "Avaric"),
    ("ay", "Aymara"),
    ("az", "Azerbaijani"),
    ("ba", "Bashkir"),
    ("be", "Belarusian"),
    ("bg", "Bulgarian"),
    ("bh", "Bihari languages"),
    ("bi", "Bislama"),
    ("bm", "Bambara"),
    ("bn", "Bangla"),
    ("bo", "Tibetan"),
    ("br", "Breton"),
    ("bs", "Bosnian"),
    ("ca", "Catalan"),
    ("ce", "Chechen"),
    ("ch", "Chamorro"),
    ("co", "Corsican"),
    ("cr", "Cree"),
    ("cs", "Czech"),
    ("cu", "Church Slavic"),
    ("cv", "Chuvash"),
    ("cy", "Welsh"),
    ("da", "Danish"),
    ("de", "German"),
    ("dv", "Divehi"),
    ("dz", "Dzongkha"),
    ("ee", "Ewe"),
    ("el", "Greek"),
    ("en", "English"),
    ("eo", "Esperanto"),
    ("es", "Spanish"),
    ("et", "Estonian"),
    ("eu", "Basque"),
    ("fa", "Persian"),
    ("ff", "Fulah"),
    ("fi", "Finnish"),
    ("fj", "Fijian"),
    ("fo", "Faroese"),
    ("fr", "French"),
    ("fy", "Western Frisian"),
    ("ga", "Irish"),
    ("gd", "Scottish Gaelic"),
    ("gl", "Galician"),
    ("gn", "Guarani"),
    ("gu", "Gujarati"),
    ("gv", "Manx"),
    ("ha", "Hausa"),
    ("he", "Hebrew"),
    ("hi", "Hindi"),
    ("ho", "Hiri Motu"),
    ("hr", "Croatian"),
    ("ht", "Haitian Creole"),
    ("hu", "Hungarian"),
    ("hy", "Armenian"),
    ("hz", "Herero"),
    ("ia", "Interlingua"),
    ("id", "Indonesian"),
    ("ie", "Interlingue"),
    ("ig", "Igbo"),
    ("ii", "Sichuan Yi"),
    ("ik", "Inupiaq"),
    ("io", "Ido"),
    ("is", "Icelandic"),
    ("it", "Italian"),
    ("iu", "Inuktitut"),
    ("ja", "Japanese"),
    ("jv", "Javanese"),
    ("ka", "Georgian"),
    ("kg", "Kongo"),
    ("ki", "Kikuyu"),
    ("kj", "Kuanyama"),
    ("kk", "Kazakh"),
    ("kl", "Kalaallisut"),
    ("km", "Khmer"),
    ("kn", "Kannada"),
    ("ko", "Korean"),
    ("kr", "Kanuri"),
    ("ks", "Kashmiri"),
    ("ku", "Kurdish"),
    ("kv", "Komi"),
    ("kw", "Cornish"),
    ("ky", "Kyrgyz"),
    ("la", "Latin"),
    ("lb", "Luxembourgish"),
    ("lg", "Ganda"),
    ("li", "Limburgish"),
    ("ln", "Lingala"),
    ("lo", "Lao"),
    ("lt", "Lithuanian"),
    ("lu", "Luba-Katanga"),
    ("lv", "Latvian"),
    ("mg", "Malagasy"),
    ("mh", "Marshallese"),
    ("mi", "Māori"),
    ("mk", "Macedonian"),
    ("ml", "Malayalam"),
    ("mn", "Mongolian"),
    ("mr", "Marathi"),
    ("ms", "Malay"),
    ("mt", "Maltese"),
    ("my", "Burmese"),
    ("na", "Nauru"),
    ("nb", "Norwegian Bokmål"),
    ("nd", "North Ndebele"),
    ("ne", "Nepali"),
    ("ng", "Ndonga"),
    ("nl", "Dutch"),
    ("nn", "Norwegian Nynorsk"),
    ("no", "Norwegian"),
    ("nr", "South Ndebele"),
    ("nv", "Navajo"),
    ("ny", "Nyanja"),
    ("oc", "Occitan"),
    ("oj", "Ojibwa"),
    ("om", "Oromo"),
    ("or", "Odia"),
    ("os", "Ossetic"),
    ("pa", "Punjabi"),
    ("pi", "Pali"),
    ("pl", "Polish"),
    ("ps", "Pashto"),
    ("pt", "Portuguese"),
    ("qu", "Quechua"),
    ("rm", "Romansh"),
    ("rn", "Rundi"),
    ("ro", "Romanian"),
    ("ru", "Russian"),
    ("rw", "Kinyarwanda"),
    ("sa", "Sanskrit"),
    ("sc", "Sardinian"),
    ("sd", "Sindhi"),
    ("se", "Northern Sami"),
    ("sg", "Sango"),
    ("si", "Sinhala"),
    ("sk", "Slovak"),
    ("sl", "Slovenian"),
    ("sm", "Samoan"),
    ("sn", "Shona"),
    ("so", "Somali"),
    ("sq", "Albanian"),
    ("sr", "Serbian"),
    ("ss", "Swati"),
    ("st", "Southern Sotho"),
    ("su", "Sundanese"),
    ("sv", "Swedish"),
    ("sw", "Swahili"),
    ("ta", "Tamil"),
    ("te", "Telugu"),
    ("tg", "Tajik"),
    ("th", "Thai"),
    ("ti", "Tigrinya"),
    ("tk", "Turkmen"),
    ("tl", "Tagalog"),
    ("tn", "Tswana"),
    ("to", "Tongan"),
    ("tr", "Turkish"),
    ("ts", "Tsonga"),
    ("tt", "Tatar"),
    ("tw", "Twi"),
    ("ty", "Tahitian"),
    ("ug", "Uyghur"),
    ("uk", "Ukrainian"),
    ("ur", "Urdu"),
    ("uz", "Uzbek"),
    ("ve", "Venda"),
    ("vi", "Vietnamese"),
    ("vo", "Volapük"),
    ("wa", "Walloon"),
    ("wo", "Wolof"),
    ("xh", "Xhosa"),
    ("yi", "Yiddish"),
    ("yo", "Yoruba"),
    ("za", "Zhuang"),
    ("zh", "Chinese"),
    ("zu", "Zulu"),
];

fn is_language_subtag(value: &str, min: usize, max: usize) -> bool {
    let len = value.len();
    (min..=max).contains(&len) && value.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Normalize and validate user-supplied language codes.
///
/// Accepts ISO 639-1/639-2 primary language subtags and BCP-47 variants such
/// as `pt-BR`, `zh-Hant`, `es-419`, and `fil-PH` so the translator plugin can
/// target languages beyond the common UI list.
pub fn normalize_language_code(language: &str) -> Result<String, String> {
    let cleaned = language.trim().replace('_', "-");
    if cleaned.is_empty() {
        return Err("Language code cannot be empty".to_string());
    }

    let mut parts = cleaned.split('-');
    let primary = parts
        .next()
        .ok_or_else(|| "Language code cannot be empty".to_string())?
        .to_ascii_lowercase();
    let primary = if primary == "jp" {
        "ja".to_string()
    } else {
        primary
    };
    if !is_language_subtag(&primary, 2, 3) || !primary.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(format!("Unsupported language code: {language}"));
    }

    let mut normalized = primary;
    for subtag in parts {
        if !is_language_subtag(subtag, 2, 8) {
            return Err(format!("Unsupported language code: {language}"));
        }
        normalized.push('-');
        normalized.push_str(&subtag.to_ascii_lowercase());
    }

    Ok(normalized)
}

/// Normalize user-facing language input, accepting either language codes or
/// common English language names from `SUPPORTED_LANGUAGES`.
pub fn normalize_language_input(language: &str) -> Result<String, String> {
    if let Ok(code) = normalize_language_code(language) {
        return Ok(code);
    }

    let cleaned = language.trim();
    SUPPORTED_LANGUAGES
        .iter()
        .find(|(code, name)| {
            code.eq_ignore_ascii_case(cleaned) || name.eq_ignore_ascii_case(cleaned)
        })
        .map(|(code, _)| (*code).to_string())
        .ok_or_else(|| format!("Unsupported language code: {language}"))
}

/// A translation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResult {
    /// Original text.
    pub original: String,
    /// Original language code.
    pub source_lang: String,
    /// Translated text.
    pub translated: String,
    /// Target language code.
    pub target_lang: String,
    /// Confidence score (0.0–1.0), if available.
    pub confidence: Option<f64>,
}

/// List supported languages.
#[tauri::command]
pub async fn list_languages() -> Vec<(String, String)> {
    SUPPORTED_LANGUAGES
        .iter()
        .map(|(code, name)| (code.to_string(), name.to_string()))
        .collect()
}

/// Translate text from source language to target language (stub).
///
/// Returns the original text with a language tag prefix. LLM-based translation
/// is not yet integrated.
#[tauri::command]
pub async fn translate_text(
    text: String,
    source_lang: String,
    target_lang: String,
) -> Result<TranslationResult, String> {
    let trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        return Err("Text cannot be empty".to_string());
    }

    let source_lang = normalize_language_input(&source_lang)
        .map_err(|_| format!("Unsupported source language: {source_lang}"))?;
    let target_lang = normalize_language_input(&target_lang)
        .map_err(|_| format!("Unsupported target language: {target_lang}"))?;

    // Stub: return original text when same language
    if source_lang == target_lang {
        return Ok(TranslationResult {
            original: trimmed.clone(),
            source_lang,
            translated: trimmed,
            target_lang,
            confidence: Some(1.0),
        });
    }

    // Stub translation: prefix with target language tag
    Ok(TranslationResult {
        original: trimmed.clone(),
        source_lang,
        translated: format!("[{target_lang}] {trimmed}"),
        target_lang,
        confidence: Some(0.5),
    })
}

/// Detect the language of text (stub).
///
/// Returns "en" (English) as the detected language. Language detection
/// via LLM or dedicated library is not yet integrated.
#[tauri::command]
pub async fn detect_language(text: String) -> Result<(String, f64), String> {
    if text.trim().is_empty() {
        return Err("Text cannot be empty".to_string());
    }
    // Stub: always returns English
    Ok(("en".to_string(), 0.9))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_languages_returns_all() {
        let langs = list_languages().await;
        assert_eq!(langs.len(), SUPPORTED_LANGUAGES.len());
        assert!(langs.iter().any(|(code, _)| code == "en"));
        assert!(langs.iter().any(|(code, _)| code == "ja"));
        assert!(langs.iter().any(|(code, _)| code == "zu"));
    }

    #[tokio::test]
    async fn translate_text_accepts_every_listed_language() {
        for (code, _) in SUPPORTED_LANGUAGES {
            let result = translate_text("Hello".into(), "en".into(), (*code).into())
                .await
                .unwrap();
            assert_eq!(result.target_lang, *code);
        }
    }

    #[tokio::test]
    async fn translate_text_accepts_worldwide_bcp47_codes() {
        for code in ["pt-BR", "zh-Hant", "fil-PH", "haw-US", "chr-US"] {
            let result = translate_text("Hello".into(), "en".into(), code.into())
                .await
                .unwrap();
            assert_eq!(result.target_lang, code.to_ascii_lowercase());
        }
    }

    #[tokio::test]
    async fn translate_text_accepts_common_language_names() {
        let result = translate_text("Hello".into(), "English".into(), "Vietnamese".into())
            .await
            .unwrap();
        assert_eq!(result.source_lang, "en");
        assert_eq!(result.target_lang, "vi");
    }

    #[tokio::test]
    async fn translate_text_same_lang_returns_original() {
        let result = translate_text("Hello".into(), "en".into(), "en".into())
            .await
            .unwrap();
        assert_eq!(result.original, "Hello");
        assert_eq!(result.translated, "Hello");
        assert_eq!(result.confidence, Some(1.0));
    }

    #[tokio::test]
    async fn translate_text_different_lang_returns_tagged() {
        let result = translate_text("Hello".into(), "en".into(), "ja".into())
            .await
            .unwrap();
        assert_eq!(result.original, "Hello");
        assert!(result.translated.starts_with("[ja]"));
        assert_eq!(result.source_lang, "en");
        assert_eq!(result.target_lang, "ja");
    }

    #[tokio::test]
    async fn translate_text_rejects_empty() {
        let result = translate_text("".into(), "en".into(), "ja".into()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn translate_text_rejects_invalid_source_lang() {
        let result = translate_text("Hello".into(), "x".into(), "en".into()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported source language"));
    }

    #[tokio::test]
    async fn translate_text_rejects_invalid_target_lang() {
        let result = translate_text("Hello".into(), "en".into(), "en-".into()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported target language"));
    }

    #[tokio::test]
    async fn detect_language_returns_stub() {
        let (lang, conf) = detect_language("Hello world".into()).await.unwrap();
        assert_eq!(lang, "en");
        assert!(conf > 0.0);
    }

    #[tokio::test]
    async fn detect_language_rejects_empty() {
        let result = detect_language("".into()).await;
        assert!(result.is_err());
    }

    #[test]
    fn translation_result_serde_roundtrip() {
        let result = TranslationResult {
            original: "Hello".into(),
            source_lang: "en".into(),
            translated: "[ja] Hello".into(),
            target_lang: "ja".into(),
            confidence: Some(0.5),
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: TranslationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.original, result.original);
        assert_eq!(parsed.translated, result.translated);
    }
}
