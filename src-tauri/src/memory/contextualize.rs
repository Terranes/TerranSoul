//! Contextual Retrieval (Anthropic 2024) — chunk-specific context injection.
//!
//! At ingest time, the LLM prepends a 50–100 token context prefix to each
//! chunk *before* it is embedded. This gives the embedding model critical
//! document-level context that would otherwise be lost when a document is
//! split into small chunks.
//!
//! Example: a raw chunk `"Article 3 allows processing when..."` becomes
//! `"This chunk is from §3 of Vietnamese Decree 13/2023 on personal data,
//! which discusses lawful processing bases.\n\nArticle 3 allows processing
//! when..."` before embedding.
//!
//! Anthropic reports a ~49 % drop in failed retrievals with this technique.
//!
//! The feature is opt-in via `AppSettings.contextual_retrieval = true` and
//! adds one LLM call per chunk at ingest time.
//!
//! Maps to `docs/brain-advanced-design.md` §19.2 row 3 (chunk 16.2).

use crate::brain::{BrainMode, OllamaAgent, OpenAiClient};
use crate::brain::ollama_agent::ChatMessage;
use crate::brain::openai_client::OpenAiMessage;

/// Build the system prompt for the contextualiser.
fn system_prompt() -> &'static str {
    "You are a document context assistant. Given a document summary and a \
     chunk of text from that document, write a SHORT context sentence (50–100 \
     tokens) that situates the chunk within the broader document. The context \
     should mention the document title/topic and which section or aspect the \
     chunk covers. Reply with ONLY the context sentence, nothing else."
}

/// Build the user prompt for contextualisation.
fn user_prompt(doc_summary: &str, chunk: &str) -> String {
    format!(
        "<document_summary>\n{doc_summary}\n</document_summary>\n\n\
         <chunk>\n{chunk}\n</chunk>"
    )
}

/// Prepend the context prefix to the original chunk content.
///
/// If the context is empty or whitespace-only, returns the chunk unchanged.
pub fn prepend_context(context: &str, chunk: &str) -> String {
    let trimmed = context.trim();
    if trimmed.is_empty() {
        return chunk.to_string();
    }
    format!("{trimmed}\n\n{chunk}")
}

/// Generate a document-level summary from the first chunk (or full text if
/// short enough). Used once per ingest to create the `doc_summary` that is
/// then passed to [`contextualise_chunk`] for every chunk.
///
/// Returns `None` when the brain is unreachable or the content is empty.
pub async fn generate_doc_summary(text: &str, brain_mode: &BrainMode) -> Option<String> {
    if text.trim().is_empty() {
        return None;
    }

    // Use at most the first 2000 chars to avoid blowing context windows.
    let preview = if text.len() > 2000 { &text[..2000] } else { text };

    let sys = "You are a document summariser. Given the beginning of a document, \
               write a 1–2 sentence summary that captures the document's title, \
               topic, and scope. Reply with ONLY the summary, nothing else.";
    let user = format!("<document_start>\n{preview}\n</document_start>");

    call_llm(sys, &user, brain_mode).await
}

/// Generate a context prefix for a single chunk.
///
/// Returns `None` when the brain is unreachable or the response is empty.
pub async fn contextualise_chunk(
    doc_summary: &str,
    chunk: &str,
    brain_mode: &BrainMode,
) -> Option<String> {
    if chunk.trim().is_empty() || doc_summary.trim().is_empty() {
        return None;
    }

    let sys = system_prompt();
    let user = user_prompt(doc_summary, chunk);

    call_llm(sys, &user, brain_mode).await
}

/// Dispatch an LLM call to the active brain mode.
///
/// Returns `None` when the brain is unreachable.
async fn call_llm(system: &str, user: &str, brain_mode: &BrainMode) -> Option<String> {
    let reply = match brain_mode {
        BrainMode::LocalOllama { model } => {
            let agent = OllamaAgent::new(model);
            let msgs = vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user.to_string(),
                },
            ];
            let (response, _) = agent.call(msgs).await;
            response
        }
        BrainMode::FreeApi {
            provider_id,
            api_key,
        } => {
            let provider = crate::brain::get_free_provider(provider_id)?;
            let client =
                OpenAiClient::new(&provider.base_url, &provider.model, api_key.as_deref());
            let msgs = vec![
                OpenAiMessage {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                OpenAiMessage {
                    role: "user".to_string(),
                    content: user.to_string(),
                },
            ];
            client.chat(msgs).await.ok()?
        }
        BrainMode::PaidApi {
            provider: _,
            api_key,
            model,
            base_url,
        } => {
            let client = OpenAiClient::new(base_url, model, Some(api_key));
            let msgs = vec![
                OpenAiMessage {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                OpenAiMessage {
                    role: "user".to_string(),
                    content: user.to_string(),
                },
            ];
            client.chat(msgs).await.ok()?
        }
    };

    let trimmed = reply.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepend_context_adds_prefix() {
        let ctx = "This chunk is from §3 of the Decree on personal data.";
        let chunk = "Article 3 allows processing when consent is given.";
        let result = prepend_context(ctx, chunk);
        assert!(result.starts_with(ctx));
        assert!(result.ends_with(chunk));
        assert!(result.contains("\n\n"));
    }

    #[test]
    fn prepend_context_empty_returns_chunk() {
        let chunk = "Article 3 allows processing when consent is given.";
        assert_eq!(prepend_context("", chunk), chunk);
        assert_eq!(prepend_context("   ", chunk), chunk);
    }

    #[test]
    fn prepend_context_trims_whitespace() {
        let ctx = "  Context with spaces.  ";
        let chunk = "Some chunk.";
        let result = prepend_context(ctx, chunk);
        assert!(result.starts_with("Context with spaces."));
    }

    #[test]
    fn system_prompt_is_non_empty() {
        assert!(!system_prompt().is_empty());
    }

    #[test]
    fn user_prompt_contains_both_parts() {
        let prompt = user_prompt("Summary of a decree.", "Article 3 text.");
        assert!(prompt.contains("Summary of a decree."));
        assert!(prompt.contains("Article 3 text."));
        assert!(prompt.contains("<document_summary>"));
        assert!(prompt.contains("<chunk>"));
    }

    #[test]
    fn user_prompt_xml_structure() {
        let prompt = user_prompt("doc", "chunk");
        // Verify proper XML-like wrapping
        assert!(prompt.contains("<document_summary>"));
        assert!(prompt.contains("</document_summary>"));
        assert!(prompt.contains("<chunk>"));
        assert!(prompt.contains("</chunk>"));
    }
}
