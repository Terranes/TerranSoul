/// Emotion/motion tag parser for LLM response text.
///
/// Parses tags like `[happy]`, `[sad]`, `[motion:wave]` from LLM text,
/// strips them from the display text, and returns the extracted metadata.
use serde::{Deserialize, Serialize};

/// An emotion tag that the LLM can embed in its response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmotionTag {
    Happy,
    Sad,
    Angry,
    Relaxed,
    Surprised,
    Neutral,
}

impl EmotionTag {
    /// Try to parse an emotion from a tag string (case-insensitive).
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "happy" => Some(EmotionTag::Happy),
            "sad" => Some(EmotionTag::Sad),
            "angry" => Some(EmotionTag::Angry),
            "relaxed" => Some(EmotionTag::Relaxed),
            "surprised" => Some(EmotionTag::Surprised),
            "neutral" => Some(EmotionTag::Neutral),
            _ => None,
        }
    }
}

/// A parsed text chunk with optional emotion and motion tags extracted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedChunk {
    /// Display text with all tags stripped.
    pub text: String,
    /// Emotion detected in this chunk, if any.
    pub emotion: Option<EmotionTag>,
    /// Motion tag detected (e.g. "wave", "nod"), if any.
    pub motion: Option<String>,
}

/// Parse emotion and motion tags from a text chunk.
///
/// Recognized tags:
/// - Emotion: `[happy]`, `[sad]`, `[angry]`, `[relaxed]`, `[surprised]`, `[neutral]`
/// - Motion:  `[motion:wave]`, `[motion:nod]`, etc.
///
/// Tags are stripped from the returned text. Only the first emotion and first
/// motion tag per chunk are used (subsequent ones are still stripped).
pub fn parse_tags(input: &str) -> ParsedChunk {
    let mut text = input.to_string();
    let mut emotion: Option<EmotionTag> = None;
    let mut motion: Option<String> = None;

    // Simple regex-free bracket parser
    loop {
        let Some(start) = text.find('[') else { break };
        let Some(end) = text[start..].find(']') else { break };
        let end = start + end;
        let tag_content = &text[start + 1..end];

        if tag_content.starts_with("motion:") {
            let motion_name = tag_content.trim_start_matches("motion:").trim();
            if motion.is_none() && !motion_name.is_empty() {
                motion = Some(motion_name.to_string());
            }
            text = format!("{}{}", &text[..start], text[end + 1..].trim_start());
        } else if let Some(em) = EmotionTag::from_str(tag_content) {
            if emotion.is_none() {
                emotion = Some(em);
            }
            text = format!("{}{}", &text[..start], text[end + 1..].trim_start());
        } else {
            // Not a recognized tag — skip past it to avoid infinite loop
            // Move past this bracket pair by replacing just the opening bracket
            // with a placeholder, process remaining, then restore
            break;
        }
    }

    ParsedChunk {
        text: text.trim().to_string(),
        emotion,
        motion,
    }
}

/// Strip all emotion/motion tags from text, returning clean display text.
pub fn strip_tags(input: &str) -> String {
    parse_tags(input).text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_happy_tag() {
        let result = parse_tags("[happy] Great to see you!");
        assert_eq!(result.emotion, Some(EmotionTag::Happy));
        assert_eq!(result.text, "Great to see you!");
        assert!(result.motion.is_none());
    }

    #[test]
    fn parse_sad_tag() {
        let result = parse_tags("[sad] I'm sorry to hear that.");
        assert_eq!(result.emotion, Some(EmotionTag::Sad));
        assert_eq!(result.text, "I'm sorry to hear that.");
    }

    #[test]
    fn parse_angry_tag() {
        let result = parse_tags("[angry] That's not right!");
        assert_eq!(result.emotion, Some(EmotionTag::Angry));
    }

    #[test]
    fn parse_relaxed_tag() {
        let result = parse_tags("[relaxed] Take it easy.");
        assert_eq!(result.emotion, Some(EmotionTag::Relaxed));
    }

    #[test]
    fn parse_surprised_tag() {
        let result = parse_tags("[surprised] Oh wow!");
        assert_eq!(result.emotion, Some(EmotionTag::Surprised));
    }

    #[test]
    fn parse_neutral_tag() {
        let result = parse_tags("[neutral] The weather is mild today.");
        assert_eq!(result.emotion, Some(EmotionTag::Neutral));
    }

    #[test]
    fn parse_motion_tag() {
        let result = parse_tags("[motion:wave] Hello there!");
        assert!(result.emotion.is_none());
        assert_eq!(result.motion, Some("wave".to_string()));
        assert_eq!(result.text, "Hello there!");
    }

    #[test]
    fn parse_both_emotion_and_motion() {
        let result = parse_tags("[happy] [motion:nod] Absolutely!");
        assert_eq!(result.emotion, Some(EmotionTag::Happy));
        assert_eq!(result.motion, Some("nod".to_string()));
        assert_eq!(result.text, "Absolutely!");
    }

    #[test]
    fn no_tags_returns_original() {
        let result = parse_tags("Just plain text.");
        assert!(result.emotion.is_none());
        assert!(result.motion.is_none());
        assert_eq!(result.text, "Just plain text.");
    }

    #[test]
    fn empty_input() {
        let result = parse_tags("");
        assert!(result.emotion.is_none());
        assert!(result.motion.is_none());
        assert_eq!(result.text, "");
    }

    #[test]
    fn case_insensitive_emotion() {
        let result = parse_tags("[Happy] Hello!");
        assert_eq!(result.emotion, Some(EmotionTag::Happy));
    }

    #[test]
    fn unrecognized_tag_preserved() {
        let result = parse_tags("[unknown] Some text.");
        // Unrecognized brackets stay in the text
        assert!(result.emotion.is_none());
        assert_eq!(result.text, "[unknown] Some text.");
    }

    #[test]
    fn first_emotion_wins() {
        let result = parse_tags("[happy] [sad] Mixed feelings.");
        assert_eq!(result.emotion, Some(EmotionTag::Happy));
        assert_eq!(result.text, "Mixed feelings.");
    }

    #[test]
    fn strip_tags_function() {
        assert_eq!(strip_tags("[happy] Hello!"), "Hello!");
        assert_eq!(strip_tags("[motion:wave] Hi!"), "Hi!");
        assert_eq!(strip_tags("No tags here"), "No tags here");
    }

    #[test]
    fn emotion_tag_serializes() {
        let json = serde_json::to_string(&EmotionTag::Happy).unwrap();
        assert_eq!(json, r#""happy""#);
    }

    #[test]
    fn emotion_tag_deserializes() {
        let tag: EmotionTag = serde_json::from_str(r#""surprised""#).unwrap();
        assert_eq!(tag, EmotionTag::Surprised);
    }

    #[test]
    fn parsed_chunk_serializes() {
        let chunk = ParsedChunk {
            text: "Hello".to_string(),
            emotion: Some(EmotionTag::Happy),
            motion: Some("wave".to_string()),
        };
        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("happy"));
        assert!(json.contains("wave"));
    }
}
