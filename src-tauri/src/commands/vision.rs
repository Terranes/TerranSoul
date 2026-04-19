use serde::{Deserialize, Serialize};

/// A captured screen frame with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenFrame {
    /// Base64-encoded image data (PNG).
    pub image_b64: String,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Timestamp when captured (Unix ms).
    pub captured_at: u64,
    /// Title of the active window, if detectable.
    pub active_window_title: Option<String>,
}

/// Result of vision analysis on a screen frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionAnalysis {
    /// Natural language description of what's on screen.
    pub description: String,
    /// Detected activity category (e.g. "coding", "browsing", "gaming", "reading").
    pub activity: String,
    /// Confidence score (0.0–1.0).
    pub confidence: f64,
    /// Timestamp of analysis.
    pub analyzed_at: u64,
}

/// Capture a screenshot of the current screen (stub).
///
/// Returns a minimal 1×1 PNG stub frame. Screen capture requires
/// platform-specific APIs that are not yet integrated.
#[tauri::command]
pub async fn capture_screen() -> Result<ScreenFrame, String> {
    // Return a minimal 1x1 PNG as a stub response
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    Ok(ScreenFrame {
        image_b64: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string(),
        width: 1,
        height: 1,
        captured_at: now,
        active_window_title: Some("TerranSoul".to_string()),
    })
}

/// Analyze a screen frame using the LLM brain (stub).
///
/// Returns a fixed analysis based on frame metadata. Vision-capable LLM
/// integration is not yet available.
#[tauri::command]
pub async fn analyze_screen(frame: ScreenFrame) -> Result<VisionAnalysis, String> {
    if frame.image_b64.is_empty() {
        return Err("Empty screen frame".to_string());
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    Ok(VisionAnalysis {
        description: format!(
            "User is working in {} ({}×{})",
            frame.active_window_title.as_deref().unwrap_or("unknown app"),
            frame.width,
            frame.height
        ),
        activity: "working".to_string(),
        confidence: 0.85,
        analyzed_at: now,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn capture_screen_returns_stub_frame() {
        let frame = capture_screen().await.unwrap();
        assert!(!frame.image_b64.is_empty());
        assert_eq!(frame.width, 1);
        assert_eq!(frame.height, 1);
        assert!(frame.captured_at > 0);
        assert_eq!(frame.active_window_title, Some("TerranSoul".to_string()));
    }

    #[tokio::test]
    async fn analyze_screen_returns_stub_analysis() {
        let frame = ScreenFrame {
            image_b64: "dGVzdA==".to_string(),
            width: 1920,
            height: 1080,
            captured_at: 1000,
            active_window_title: Some("VS Code".to_string()),
        };
        let analysis = analyze_screen(frame).await.unwrap();
        assert!(analysis.description.contains("VS Code"));
        assert_eq!(analysis.activity, "working");
        assert!(analysis.confidence > 0.0);
        assert!(analysis.analyzed_at > 0);
    }

    #[tokio::test]
    async fn analyze_screen_rejects_empty_frame() {
        let frame = ScreenFrame {
            image_b64: "".to_string(),
            width: 0,
            height: 0,
            captured_at: 0,
            active_window_title: None,
        };
        let result = analyze_screen(frame).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn analyze_screen_handles_no_window_title() {
        let frame = ScreenFrame {
            image_b64: "dGVzdA==".to_string(),
            width: 800,
            height: 600,
            captured_at: 1000,
            active_window_title: None,
        };
        let analysis = analyze_screen(frame).await.unwrap();
        assert!(analysis.description.contains("unknown app"));
    }

    #[test]
    fn screen_frame_serde_roundtrip() {
        let frame = ScreenFrame {
            image_b64: "abc123".to_string(),
            width: 1920,
            height: 1080,
            captured_at: 12345,
            active_window_title: Some("Firefox".to_string()),
        };
        let json = serde_json::to_string(&frame).unwrap();
        let parsed: ScreenFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.image_b64, frame.image_b64);
        assert_eq!(parsed.width, frame.width);
        assert_eq!(parsed.active_window_title, frame.active_window_title);
    }

    #[test]
    fn vision_analysis_serde_roundtrip() {
        let analysis = VisionAnalysis {
            description: "User coding in VS Code".to_string(),
            activity: "coding".to_string(),
            confidence: 0.92,
            analyzed_at: 54321,
        };
        let json = serde_json::to_string(&analysis).unwrap();
        let parsed: VisionAnalysis = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.description, analysis.description);
        assert_eq!(parsed.activity, analysis.activity);
        assert!((parsed.confidence - analysis.confidence).abs() < f64::EPSILON);
    }
}
