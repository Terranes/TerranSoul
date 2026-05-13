// SPDX-License-Identifier: MIT
//! Privacy scrubbing for memory content.
//!
//! Strips common secret patterns (API keys, tokens, passwords) from text
//! before it is stored in long-term memory. Inspired by the privacy-scrub
//! approach in the open-source agentmemory project (credited in CREDITS.md).

use std::sync::LazyLock;

/// Replacement marker for redacted secrets.
const REDACTED: &str = "[REDACTED]";

// ---------------------------------------------------------------------------
// Prefix-based token patterns (most common API key formats)
// ---------------------------------------------------------------------------

/// Known secret prefixes — (prefix, min_suffix_len).
/// If a word starts with the prefix and has at least `min_suffix_len` extra
/// characters, it is considered a secret.
static PREFIX_PATTERNS: LazyLock<Vec<(&'static str, usize)>> = LazyLock::new(|| {
    vec![
        ("sk-ant-", 20),     // Anthropic
        ("sk-", 20),         // OpenAI
        ("ghp_", 20),        // GitHub PAT
        ("gho_", 20),        // GitHub OAuth
        ("ghu_", 20),        // GitHub user-to-server
        ("ghs_", 20),        // GitHub server-to-server
        ("github_pat_", 20), // GitHub fine-grained PAT
        ("glpat-", 12),      // GitLab PAT
        ("xoxb-", 20),       // Slack bot
        ("xoxp-", 20),       // Slack user
        ("xapp-", 20),       // Slack app-level
        ("AKIA", 16),        // AWS access key
        ("AIza", 30),        // Google Cloud API key
        ("npm_", 30),        // npm token
        ("pypi-", 30),       // PyPI token
        ("sq0csp-", 20),     // Square sandbox
        ("sq0atp-", 20),     // Square production
        ("shpat_", 20),      // Shopify
        ("hf_", 20),         // HuggingFace
    ]
});

/// Scrub prefix-based secret tokens from text.
fn scrub_prefixed_tokens(input: &str) -> String {
    let mut result = input.to_string();
    for (prefix, min_suffix) in PREFIX_PATTERNS.iter() {
        while let Some(start) = result.find(prefix) {
            // Find the end of the token (next whitespace, quote, comma, or end)
            let token_start = start;
            let after_prefix = start + prefix.len();
            let rest = &result[after_prefix..];
            let token_len = rest
                .find(|c: char| {
                    c.is_whitespace()
                        || c == '"'
                        || c == '\''
                        || c == ','
                        || c == ';'
                        || c == ')'
                        || c == '}'
                        || c == ']'
                })
                .unwrap_or(rest.len());
            if token_len >= *min_suffix {
                let token_end = after_prefix + token_len;
                result.replace_range(token_start..token_end, REDACTED);
            } else {
                // Not long enough — skip this occurrence by inserting a marker
                // to avoid infinite loop. Replace just the prefix with itself
                // plus a zero-width char we'll remove later.
                break;
            }
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Key-value patterns (password=..., api_key: ..., etc.)
// ---------------------------------------------------------------------------

/// Key names that signal a secret value follows.
static SECRET_KEY_NAMES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    vec![
        "password",
        "passwd",
        "api_key",
        "apikey",
        "api-key",
        "secret",
        "secret_key",
        "access_token",
        "auth_token",
        "bearer",
        "authorization",
        "private_key",
        "client_secret",
        "database_url",
        "connection_string",
    ]
});

/// Scrub key=value or key: value pairs where key is a secret name.
fn scrub_key_value_pairs(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for line in input.lines() {
        let lower = line.to_lowercase();
        let mut scrubbed = false;
        for key in SECRET_KEY_NAMES.iter() {
            // Match patterns: key=value, key = value, key: value, key : value
            // Also "key" = value, 'key' = value
            let clean_key = key.to_lowercase();
            if let Some(key_pos) = lower.find(&clean_key) {
                let after_key = key_pos + clean_key.len();
                let rest = lower[after_key..].trim_start();
                if rest.starts_with('=') || rest.starts_with(':') {
                    // Found a key-value pair — redact the value portion
                    let original_after_key = line[after_key..].trim_start();
                    if let Some(sep_pos) = original_after_key.find(['=', ':']) {
                        let value_start_in_rest = sep_pos + 1;
                        let value_part = original_after_key[value_start_in_rest..].trim_start();
                        if !value_part.is_empty() {
                            // Reconstruct: everything up to value + REDACTED
                            let abs_value_start = after_key
                                + (original_after_key.len()
                                    - original_after_key.trim_start().len())
                                + value_start_in_rest
                                + (value_part.len()
                                    - original_after_key[value_start_in_rest..].trim_start().len());
                            let prefix_part = &line[..abs_value_start];
                            result.push_str(prefix_part);
                            result.push_str(REDACTED);
                            scrubbed = true;
                            break;
                        }
                    }
                }
            }
        }
        if !scrubbed {
            result.push_str(line);
        }
        result.push('\n');
    }
    // Remove trailing newline if original didn't have one
    if !input.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }
    result
}

// ---------------------------------------------------------------------------
// JWT pattern: eyJ...<base64>.<base64>.<base64>
// ---------------------------------------------------------------------------

fn contains_jwt(s: &str) -> bool {
    s.contains("eyJ")
}

fn scrub_jwt(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut i = 0;
    let bytes = input.as_bytes();
    while i < bytes.len() {
        if i + 4 <= bytes.len() && &input[i..i + 3] == "eyJ" {
            // Potential JWT — look for xxx.yyy.zzz pattern (all base64url chars + dots)
            let start = i;
            let mut dots = 0;
            let mut j = i;
            while j < bytes.len() {
                let c = bytes[j] as char;
                if c == '.' {
                    dots += 1;
                    j += 1;
                } else if c.is_alphanumeric()
                    || c == '_'
                    || c == '-'
                    || c == '+'
                    || c == '/'
                    || c == '='
                {
                    j += 1;
                } else {
                    break;
                }
            }
            if dots >= 2 && (j - start) > 30 {
                result.push_str(REDACTED);
                i = j;
            } else {
                result.push(bytes[i] as char);
                i += 1;
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Explicit <private>...</private> tags (user-marked redactions)
// ---------------------------------------------------------------------------

fn scrub_private_tags(input: &str) -> String {
    let mut result = input.to_string();
    while let (Some(open), _) = (result.find("<private>"), result.find("</private>")) {
        if let Some(close) = result[open..].find("</private>") {
            let end = open + close + "</private>".len();
            result.replace_range(open..end, REDACTED);
        } else {
            break;
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Strip secrets and sensitive patterns from memory content before storage.
///
/// Applies the following scrubbing passes (order matters):
/// 1. Explicit `<private>` tags
/// 2. Prefix-based API key/token patterns
/// 3. JWT tokens
/// 4. Key-value pairs with sensitive key names
///
/// Returns the scrubbed content. If no secrets are found, returns the
/// original string unmodified.
pub fn strip_secrets(content: &str) -> String {
    if content.is_empty() {
        return content.to_string();
    }

    let mut result = scrub_private_tags(content);
    result = scrub_prefixed_tokens(&result);
    if contains_jwt(&result) {
        result = scrub_jwt(&result);
    }
    result = scrub_key_value_pairs(&result);
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_openai_key() {
        let input = "Use key sk-proj-abc123def456ghi789jkl012mno345pqr678 for API";
        let result = strip_secrets(input);
        assert!(!result.contains("sk-proj-"));
        assert!(result.contains(REDACTED));
        assert!(result.contains("Use key"));
    }

    #[test]
    fn strips_anthropic_key() {
        let input = "export ANTHROPIC_API_KEY=sk-ant-api03-abcdefghijklmnopqrstuvwxyz";
        let result = strip_secrets(input);
        assert!(!result.contains("sk-ant-"));
        assert!(result.contains(REDACTED));
    }

    #[test]
    fn strips_github_pat() {
        let input = "token: ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdef12345";
        let result = strip_secrets(input);
        assert!(!result.contains("ghp_"));
        assert!(result.contains(REDACTED));
    }

    #[test]
    fn strips_aws_key() {
        let input = "aws_access_key_id = AKIAIOSFODNN7EXAMPLE";
        let result = strip_secrets(input);
        assert!(!result.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(result.contains(REDACTED));
    }

    #[test]
    fn strips_jwt_token() {
        let input = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let result = strip_secrets(input);
        assert!(!result.contains("eyJ"));
        assert!(result.contains(REDACTED));
    }

    #[test]
    fn strips_password_in_key_value() {
        let input = "database password = mysecretpass123";
        let result = strip_secrets(input);
        assert!(!result.contains("mysecretpass123"));
        assert!(result.contains(REDACTED));
    }

    #[test]
    fn strips_private_tags() {
        let input = "My name is <private>John Doe</private> and I live here";
        let result = strip_secrets(input);
        assert!(!result.contains("John Doe"));
        assert!(result.contains(REDACTED));
        assert!(result.contains("My name is"));
    }

    #[test]
    fn preserves_safe_content() {
        let input = "TerranSoul uses Vue 3 and Tauri 2 for its UI layer";
        let result = strip_secrets(input);
        assert_eq!(result, input);
    }

    #[test]
    fn strips_huggingface_token() {
        let input = "HF_TOKEN=hf_abcdefghijklmnopqrstuvwxyz1234567890";
        let result = strip_secrets(input);
        assert!(!result.contains("hf_abcdefghijklmnopqrst"));
        assert!(result.contains(REDACTED));
    }

    #[test]
    fn strips_multiple_secrets_in_same_text() {
        let input =
            "key1: sk-ant-api03-aaabbbcccdddeeefffggghhh and also ghp_1234567890abcdefghijklmnop";
        let result = strip_secrets(input);
        assert!(!result.contains("sk-ant-"));
        assert!(!result.contains("ghp_"));
        assert_eq!(result.matches(REDACTED).count(), 2);
    }

    #[test]
    fn strips_gitlab_pat() {
        let input = "GITLAB_TOKEN=glpat-xxxxxxxxxxxxxxxxxxxx";
        let result = strip_secrets(input);
        assert!(!result.contains("glpat-"));
        assert!(result.contains(REDACTED));
    }

    #[test]
    fn empty_input_returns_empty() {
        assert_eq!(strip_secrets(""), "");
    }
}
