//! Tauri commands for self-improve coding sessions
//! (Chunk 30.2 — chat history + session management absorbed from
//! claw-code / Claude Code / OpenClaw).
//!
//! These commands sit on top of the existing
//! [`coding::handoff_store`](crate::coding::handoff_store) (per-session
//! handoff snapshots) and the new
//! [`coding::session_chat`](crate::coding::session_chat) (per-session
//! transcript JSONL). The frontend self-improve sessions sidebar drives
//! the entire UX through this surface.

use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::Path};
use tauri::State;

use crate::coding::{self, ChatMessage, ChatSummary, HandoffSummary};
use crate::AppState;

/// Combined sidebar row: `HandoffSummary` plus the cheap `ChatSummary`
/// so the UI can render both in one round-trip per refresh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingSessionEntry {
    /// Same fields as [`coding::HandoffSummary`].
    #[serde(flatten)]
    pub handoff: HandoffSummary,
    /// Cheap chat-side summary (count + last user preview).
    pub chat: ChatSummary,
}

/// Pure helper used by [`coding_session_list`] and unit tests.
fn collect_session_entries(data_dir: &Path) -> Result<Vec<CodingSessionEntry>, String> {
    let handoffs = coding::list_handoffs(data_dir)?;
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(handoffs.len());
    for h in handoffs {
        seen.insert(h.session_id.clone());
        let chat = coding::session_chat_summary(data_dir, &h.session_id).unwrap_or_default();
        out.push(CodingSessionEntry { handoff: h, chat });
    }
    collect_chat_only_entries(data_dir, &seen, &mut out)?;
    out.sort_by_key(|entry| {
        std::cmp::Reverse(entry.handoff.modified_at.max(entry.chat.modified_at))
    });
    Ok(out)
}

fn collect_chat_only_entries(
    data_dir: &Path,
    seen_handoffs: &HashSet<String>,
    out: &mut Vec<CodingSessionEntry>,
) -> Result<(), String> {
    let dir = coding::handoff_store::sessions_dir(data_dir);
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&dir).map_err(|e| format!("read sessions dir: {e}"))? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(session_id) = file_name.strip_suffix(".chat.jsonl") else {
            continue;
        };
        if seen_handoffs.contains(session_id) {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let chat = coding::session_chat_summary(data_dir, session_id).unwrap_or_default();
        out.push(CodingSessionEntry {
            handoff: HandoffSummary {
                session_id: session_id.to_string(),
                chunk_id: String::new(),
                last_action: transcript_last_action(data_dir, session_id),
                created_at: chat.modified_at,
                modified_at: chat.modified_at,
                bytes: metadata.len(),
            },
            chat,
        });
    }
    Ok(())
}

fn transcript_last_action(data_dir: &Path, session_id: &str) -> String {
    coding::session_chat_load(data_dir, session_id, Some(1))
        .ok()
        .and_then(|messages| messages.into_iter().last())
        .map(|msg| format!("{}: {}", msg.role, preview_text(&msg.content, 120)))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Transcript-only session".to_string())
}

fn preview_text(s: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i >= max_chars {
            out.push('…');
            break;
        }
        if ch == '\n' || ch == '\r' {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out
}

/// Pure helper for the rename flow. Copies handoff + transcript under
/// the new id, then best-effort cleans up the old slot. Returns the
/// number of messages migrated.
fn rename_session(
    data_dir: &Path,
    session_id: &str,
    new_session_id: &str,
) -> Result<usize, String> {
    if session_id == new_session_id {
        return Err("new id is identical to current id".to_string());
    }
    let prior = coding::load_handoff(data_dir, session_id)?;
    let copied = coding::session_chat_fork(data_dir, session_id, new_session_id)?;
    if let Some(mut snap) = prior {
        snap.session_id = new_session_id.to_string();
        coding::save_handoff(data_dir, &snap)?;
    }
    if let Err(e) = coding::clear_handoff(data_dir, session_id) {
        eprintln!("[coding-session] clear old handoff {session_id} failed: {e}");
    }
    if let Err(e) = coding::session_chat_clear(data_dir, session_id) {
        eprintln!("[coding-session] clear old chat {session_id} failed: {e}");
    }
    Ok(copied)
}

/// Pure helper for the fork flow. Copies handoff + transcript under
/// the new id without touching the source.
fn fork_session(data_dir: &Path, session_id: &str, new_session_id: &str) -> Result<usize, String> {
    if session_id == new_session_id {
        return Err("fork target is identical to source".to_string());
    }
    let copied = coding::session_chat_fork(data_dir, session_id, new_session_id)?;
    if let Some(mut snap) = coding::load_handoff(data_dir, session_id)? {
        snap.session_id = new_session_id.to_string();
        coding::save_handoff(data_dir, &snap)?;
    }
    Ok(copied)
}

fn purge_session(data_dir: &Path, session_id: &str) -> Result<bool, String> {
    let a = coding::clear_handoff(data_dir, session_id)?;
    let b = coding::session_chat_clear(data_dir, session_id)?;
    Ok(a || b)
}

/// List every persisted session, newest-first, with chat summaries.
#[tauri::command]
pub async fn coding_session_list(
    state: State<'_, AppState>,
) -> Result<Vec<CodingSessionEntry>, String> {
    collect_session_entries(&state.data_dir)
}

/// Append a single message to `sessionId`'s transcript. Used by the
/// frontend after every user/assistant turn.
#[tauri::command(rename_all = "camelCase")]
pub async fn coding_session_append_message(
    session_id: String,
    message: ChatMessage,
    state: State<'_, AppState>,
) -> Result<(), String> {
    coding::session_chat_append(&state.data_dir, &session_id, &message)
}

/// Load up to `limit` of the most-recent messages for `sessionId`.
/// `limit = None` falls back to the module's default cap.
#[tauri::command(rename_all = "camelCase")]
pub async fn coding_session_load_chat(
    session_id: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<ChatMessage>, String> {
    coding::session_chat_load(&state.data_dir, &session_id, limit)
}

/// Wipe the transcript for `sessionId`. Idempotent. Does **not** touch
/// the handoff snapshot — call [`coding_session_purge`] for that.
#[tauri::command(rename_all = "camelCase")]
pub async fn coding_session_clear_chat(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    coding::session_chat_clear(&state.data_dir, &session_id)
}

/// Rename `sessionId` to `newSessionId`. Implemented as copy-then-delete
/// so a partial failure never destroys the source.
#[tauri::command(rename_all = "camelCase")]
pub async fn coding_session_rename(
    session_id: String,
    new_session_id: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    rename_session(&state.data_dir, &session_id, &new_session_id)
}

/// Fork `sessionId` into `newSessionId` (Claude Code `--fork-session`).
/// Copies both the handoff snapshot and the transcript without touching
/// the source.
#[tauri::command(rename_all = "camelCase")]
pub async fn coding_session_fork(
    session_id: String,
    new_session_id: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    fork_session(&state.data_dir, &session_id, &new_session_id)
}

/// Wipe both the handoff snapshot and the transcript for `sessionId`.
/// Returns `true` when at least one of the two files was removed.
#[tauri::command(rename_all = "camelCase")]
pub async fn coding_session_purge(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    purge_session(&state.data_dir, &session_id)
}

/// Resume result returned by [`coding_session_resume`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodingSessionResumeResult {
    /// The resolved session entry (handoff + chat summary).
    pub entry: CodingSessionEntry,
    /// The last N chat messages for immediate context.
    pub messages: Vec<ChatMessage>,
}

/// Resolve a memorable name (or session ID) and return the session entry
/// plus the last N messages for context. This is the primary entry point
/// for `--resume <name>` in the headless runner and for frontend
/// "resume session" UX.
#[tauri::command(rename_all = "camelCase")]
pub async fn coding_session_resume(
    name: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<CodingSessionResumeResult, String> {
    resume_session_inner(&state.data_dir, &name, limit)
}

/// Core logic for session resume, testable without AppHandle.
fn resume_session_inner(
    data_dir: &std::path::Path,
    name: &str,
    limit: Option<usize>,
) -> Result<CodingSessionResumeResult, String> {
    let message_limit = limit.unwrap_or(50);

    // Try resolving as a memorable name first, then fall back to using
    // the name as a literal session ID.
    let session_id = if let Ok(Some(entry)) =
        coding::session_registry::resolve(data_dir, name)
    {
        entry.session_id
    } else {
        // Check if it's a valid direct session ID (handoff or chat exists).
        let has_handoff = coding::load_handoff(data_dir, name)
            .ok()
            .flatten()
            .is_some();
        let has_chat = coding::session_chat_summary(data_dir, name)
            .map(|s| s.message_count > 0)
            .unwrap_or(false);
        if has_handoff || has_chat {
            name.to_string()
        } else {
            return Err(format!(
                "session '{}' not found (checked registry and direct ID)",
                name
            ));
        }
    };

    let handoff = coding::load_handoff(data_dir, &session_id)?
        .map(|h| HandoffSummary {
            session_id: h.session_id.clone(),
            chunk_id: h.chunk_id.clone(),
            last_action: h.last_action.clone(),
            created_at: h.created_at,
            modified_at: h.created_at,
            bytes: 0,
        })
        .unwrap_or_else(|| HandoffSummary {
            session_id: session_id.clone(),
            chunk_id: String::new(),
            last_action: String::new(),
            created_at: 0,
            modified_at: 0,
            bytes: 0,
        });

    let chat = coding::session_chat_summary(data_dir, &session_id).unwrap_or_default();
    let messages = coding::session_chat_load(data_dir, &session_id, Some(message_limit))
        .unwrap_or_default();

    Ok(CodingSessionResumeResult {
        entry: CodingSessionEntry { handoff, chat },
        messages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::HandoffState;

    fn tmp_dir(tag: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "ts-coding-session-cmd-{}-{}-{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn sample_handoff(id: &str) -> HandoffState {
        let mut s = HandoffState::new(id, "30.2");
        s.last_action = "wired sessions".into();
        s.summary = "test".into();
        s.created_at = 1;
        s
    }

    #[test]
    fn list_combines_handoff_and_chat_summaries() {
        let dir = tmp_dir("list");
        coding::save_handoff(&dir, &sample_handoff("alpha")).unwrap();
        coding::session_chat_append(&dir, "alpha", &ChatMessage::now("user", "hi from test"))
            .unwrap();

        let entries = collect_session_entries(&dir).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].handoff.session_id, "alpha");
        assert_eq!(entries[0].chat.message_count, 1);
        assert_eq!(entries[0].chat.last_user_preview, "hi from test");
    }

    #[test]
    fn list_includes_chat_only_transcripts() {
        let dir = tmp_dir("chat-only");
        coding::session_chat_append(
            &dir,
            "run-only",
            &ChatMessage::now_with_kind("system", "[30.6] plan: ready", "run"),
        )
        .unwrap();

        let entries = collect_session_entries(&dir).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].handoff.session_id, "run-only");
        assert_eq!(entries[0].handoff.chunk_id, "");
        assert!(entries[0].handoff.last_action.contains("plan: ready"));
        assert_eq!(entries[0].chat.message_count, 1);
    }

    #[test]
    fn rename_moves_both_handoff_and_chat() {
        let dir = tmp_dir("rename");
        coding::save_handoff(&dir, &sample_handoff("src")).unwrap();
        coding::session_chat_append(&dir, "src", &ChatMessage::now("user", "a")).unwrap();
        coding::session_chat_append(&dir, "src", &ChatMessage::now("assistant", "b")).unwrap();

        let copied = rename_session(&dir, "src", "dst").unwrap();
        assert_eq!(copied, 2);

        assert_eq!(
            coding::session_chat_load(&dir, "dst", None).unwrap().len(),
            2
        );
        assert!(coding::load_handoff(&dir, "dst").unwrap().is_some());
        assert!(coding::load_handoff(&dir, "src").unwrap().is_none());
        assert!(coding::session_chat_load(&dir, "src", None)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn rename_to_same_id_errors() {
        let dir = tmp_dir("rename-same");
        let err = rename_session(&dir, "x", "x").unwrap_err();
        assert!(err.contains("identical"));
    }

    #[test]
    fn fork_copies_without_destroying_source() {
        let dir = tmp_dir("fork");
        coding::save_handoff(&dir, &sample_handoff("src")).unwrap();
        coding::session_chat_append(&dir, "src", &ChatMessage::now("user", "hi")).unwrap();

        let copied = fork_session(&dir, "src", "dst").unwrap();
        assert_eq!(copied, 1);
        assert!(coding::load_handoff(&dir, "src").unwrap().is_some());
        assert_eq!(
            coding::session_chat_load(&dir, "src", None).unwrap().len(),
            1
        );
    }

    #[test]
    fn fork_to_same_id_errors() {
        let dir = tmp_dir("fork-same");
        let err = fork_session(&dir, "x", "x").unwrap_err();
        assert!(err.contains("identical"));
    }

    #[test]
    fn purge_removes_handoff_and_chat() {
        let dir = tmp_dir("purge");
        coding::save_handoff(&dir, &sample_handoff("doomed")).unwrap();
        coding::session_chat_append(&dir, "doomed", &ChatMessage::now("user", "x")).unwrap();

        assert!(purge_session(&dir, "doomed").unwrap());
        assert!(coding::load_handoff(&dir, "doomed").unwrap().is_none());
        assert!(coding::session_chat_load(&dir, "doomed", None)
            .unwrap()
            .is_empty());
        // Idempotent.
        assert!(!purge_session(&dir, "doomed").unwrap());
    }

    #[test]
    fn resume_resolves_memorable_name() {
        let dir = tmp_dir("resume");
        // Create a session with a handoff + chat
        coding::save_handoff(&dir, &sample_handoff("sess-abc")).unwrap();
        coding::session_chat_append(&dir, "sess-abc", &ChatMessage::now("user", "hello"))
            .unwrap();
        coding::session_chat_append(
            &dir,
            "sess-abc",
            &ChatMessage::now("assistant", "world"),
        )
        .unwrap();

        // Register a memorable name
        let mut reg = crate::coding::session_registry::load(&dir).unwrap();
        reg.sessions.insert(
            crate::coding::session_names::normalize("swift-tiger"),
            crate::coding::session_registry::SessionEntry {
                session_id: "sess-abc".to_string(),
                created_at: 1,
            },
        );
        crate::coding::session_registry::save(&dir, &reg).unwrap();

        // Resume by memorable name
        let result = resume_session_inner(&dir, "swift-tiger", Some(50)).unwrap();
        assert_eq!(result.entry.handoff.session_id, "sess-abc");
        assert_eq!(result.messages.len(), 2);
    }

    #[test]
    fn resume_falls_back_to_direct_session_id() {
        let dir = tmp_dir("resume-direct");
        coding::save_handoff(&dir, &sample_handoff("direct-id")).unwrap();
        coding::session_chat_append(&dir, "direct-id", &ChatMessage::now("user", "test"))
            .unwrap();

        let result = resume_session_inner(&dir, "direct-id", None).unwrap();
        assert_eq!(result.entry.handoff.session_id, "direct-id");
        assert_eq!(result.messages.len(), 1);
    }

    #[test]
    fn resume_unknown_name_errors() {
        let dir = tmp_dir("resume-unknown");
        let err = resume_session_inner(&dir, "nonexistent", None).unwrap_err();
        assert!(err.contains("nonexistent"));
    }
}
