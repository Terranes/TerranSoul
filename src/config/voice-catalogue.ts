/**
 * Curated catalogue of well-known TTS voice ids users can pick from for
 * the persona/model "Provider voice" field.
 *
 * The field is provider-agnostic — the same string is passed to whichever
 * TTS provider is active. Edge-style neural voice ids (e.g.
 * `en-US-AnaNeural`) are recognised by:
 *  - the Edge TTS Rust shim,
 *  - the browser `speechSynthesis` fallback (matched by language hint),
 *  - any future OpenAI-compatible TTS that accepts a voice-name string.
 *
 * Voices are grouped by language for the `<optgroup>` rendering; the
 * `description` is shown in the suggestion box when the user types.
 *
 * Keep this list short and well-known. Users can still type any custom
 * voice id — the catalogue is exposed via `<datalist>`, not an
 * exclusive `<select>`.
 */

export interface VoiceCatalogueEntry {
  /** Voice id passed to the TTS provider. */
  id: string;
  /** Human-readable label for the dropdown. */
  label: string;
  /** Language tag (BCP-47). */
  lang: string;
  /** Coarse gender hint. */
  gender: 'female' | 'male';
}

export const VOICE_CATALOGUE: VoiceCatalogueEntry[] = [
  // ── English (US) ────────────────────────────────────────────────────────
  { id: 'en-US-AnaNeural',     label: 'Ana (female, US)',     lang: 'en-US', gender: 'female' },
  { id: 'en-US-AriaNeural',    label: 'Aria (female, US)',    lang: 'en-US', gender: 'female' },
  { id: 'en-US-JennyNeural',   label: 'Jenny (female, US)',   lang: 'en-US', gender: 'female' },
  { id: 'en-US-MichelleNeural', label: 'Michelle (female, US)', lang: 'en-US', gender: 'female' },
  { id: 'en-US-AndrewNeural',  label: 'Andrew (male, US)',    lang: 'en-US', gender: 'male' },
  { id: 'en-US-BrianNeural',   label: 'Brian (male, US)',     lang: 'en-US', gender: 'male' },
  { id: 'en-US-GuyNeural',     label: 'Guy (male, US)',       lang: 'en-US', gender: 'male' },

  // ── English (UK) ────────────────────────────────────────────────────────
  { id: 'en-GB-SoniaNeural',   label: 'Sonia (female, UK)',   lang: 'en-GB', gender: 'female' },
  { id: 'en-GB-LibbyNeural',   label: 'Libby (female, UK)',   lang: 'en-GB', gender: 'female' },
  { id: 'en-GB-RyanNeural',    label: 'Ryan (male, UK)',      lang: 'en-GB', gender: 'male' },

  // ── English (other accents) ────────────────────────────────────────────
  { id: 'en-AU-NatashaNeural', label: 'Natasha (female, AU)', lang: 'en-AU', gender: 'female' },
  { id: 'en-AU-WilliamNeural', label: 'William (male, AU)',   lang: 'en-AU', gender: 'male' },
  { id: 'en-CA-ClaraNeural',   label: 'Clara (female, CA)',   lang: 'en-CA', gender: 'female' },
  { id: 'en-IE-EmilyNeural',   label: 'Emily (female, IE)',   lang: 'en-IE', gender: 'female' },
  { id: 'en-IN-NeerjaNeural',  label: 'Neerja (female, IN)',  lang: 'en-IN', gender: 'female' },

  // ── Japanese ────────────────────────────────────────────────────────────
  { id: 'ja-JP-NanamiNeural',  label: 'Nanami (female, JP)',  lang: 'ja-JP', gender: 'female' },
  { id: 'ja-JP-KeitaNeural',   label: 'Keita (male, JP)',     lang: 'ja-JP', gender: 'male' },

  // ── Spanish ─────────────────────────────────────────────────────────────
  { id: 'es-ES-ElviraNeural',  label: 'Elvira (female, ES)',  lang: 'es-ES', gender: 'female' },
  { id: 'es-MX-DaliaNeural',   label: 'Dalia (female, MX)',   lang: 'es-MX', gender: 'female' },

  // ── French ──────────────────────────────────────────────────────────────
  { id: 'fr-FR-DeniseNeural',  label: 'Denise (female, FR)',  lang: 'fr-FR', gender: 'female' },
  { id: 'fr-FR-HenriNeural',   label: 'Henri (male, FR)',     lang: 'fr-FR', gender: 'male' },

  // ── German ──────────────────────────────────────────────────────────────
  { id: 'de-DE-KatjaNeural',   label: 'Katja (female, DE)',   lang: 'de-DE', gender: 'female' },
  { id: 'de-DE-ConradNeural',  label: 'Conrad (male, DE)',    lang: 'de-DE', gender: 'male' },

  // ── Italian / Portuguese ───────────────────────────────────────────────
  { id: 'it-IT-ElsaNeural',    label: 'Elsa (female, IT)',    lang: 'it-IT', gender: 'female' },
  { id: 'pt-BR-FranciscaNeural', label: 'Francisca (female, BR)', lang: 'pt-BR', gender: 'female' },

  // ── Korean ──────────────────────────────────────────────────────────────
  { id: 'ko-KR-SunHiNeural',   label: 'SunHi (female, KR)',   lang: 'ko-KR', gender: 'female' },
];

/** Voice-catalogue id list for quick `includes()` checks. */
export const VOICE_CATALOGUE_IDS: ReadonlySet<string> =
  new Set(VOICE_CATALOGUE.map((entry) => entry.id));
