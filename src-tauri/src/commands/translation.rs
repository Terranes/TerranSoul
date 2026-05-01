use serde::{Deserialize, Serialize};

/// Supported language codes (ISO 639-1).
/// The stub translator echoes input; real translators use LLM or API.
pub const SUPPORTED_LANGUAGES: &[(&str, &str)] = &[
    ("en", "English"),
    ("es", "Spanish"),
    ("fr", "French"),
    ("de", "German"),
    ("ja", "Japanese"),
    ("ko", "Korean"),
    ("zh", "Chinese"),
    ("vi", "Vietnamese"),
    ("it", "Italian"),
    ("th", "Thai"),
    ("pt", "Portuguese"),
    ("ru", "Russian"),
    ("ar", "Arabic"),
];

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

    // Validate language codes
    let valid_codes: Vec<&str> = SUPPORTED_LANGUAGES.iter().map(|(c, _)| *c).collect();
    if !valid_codes.contains(&source_lang.as_str()) {
        return Err(format!("Unsupported source language: {source_lang}"));
    }
    if !valid_codes.contains(&target_lang.as_str()) {
        return Err(format!("Unsupported target language: {target_lang}"));
    }

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
        let result = translate_text("Hello".into(), "xx".into(), "en".into()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported source language"));
    }

    #[tokio::test]
    async fn translate_text_rejects_invalid_target_lang() {
        let result = translate_text("Hello".into(), "en".into(), "xx".into()).await;
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
