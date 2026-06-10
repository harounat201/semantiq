use serde_json::Value;

/// Extract the semantic query text from raw input.
/// Handles plain strings and JSON payloads (OpenAI / Anthropic message formats).
pub fn normalize(raw: &str) -> String {
    let trimmed = raw.trim();

    if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
        // flat key priority: query → content → prompt → text
        for key in &["query", "content", "prompt", "text"] {
            if let Some(s) = val.get(key).and_then(|v| v.as_str()) {
                let s = s.trim();
                if !s.is_empty() {
                    return s.to_lowercase();
                }
            }
        }

        // Anthropic / OpenAI messages array: last user message wins
        if let Some(msgs) = val.get("messages").and_then(|v| v.as_array()) {
            for msg in msgs.iter().rev() {
                if msg.get("role").and_then(|r| r.as_str()) == Some("user") {
                    if let Some(s) = msg.get("content").and_then(|c| c.as_str()) {
                        let s = s.trim();
                        if !s.is_empty() {
                            return s.to_lowercase();
                        }
                    }
                }
            }
        }
    }

    // not JSON — normalize the raw string
    trimmed
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_string_lowercased_and_stripped() {
        assert_eq!(normalize("  Hello, World!  "), "hello world");
    }

    #[test]
    fn json_query_field_extracted() {
        let input = r#"{"query": "What is Rust?"}"#;
        assert_eq!(normalize(input), "what is rust?");
    }

    #[test]
    fn anthropic_messages_format() {
        let input = r#"{"messages": [{"role": "user", "content": "Explain ownership"}]}"#;
        assert_eq!(normalize(input), "explain ownership");
    }

    #[test]
    fn last_user_message_wins() {
        let input = r#"{"messages": [
            {"role": "user", "content": "First question"},
            {"role": "assistant", "content": "Answer"},
            {"role": "user", "content": "Follow-up question"}
        ]}"#;
        assert_eq!(normalize(input), "follow-up question");
    }
}
