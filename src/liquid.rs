use serde_json::{json, Value};
use tokio::process::Command;

use crate::formatter::ContentKind;

pub const LIQUID_DECISIONS_URL: &str = "https://api.liquid.ai/decisions/v1/systemone";
pub const LIQUID_MODEL: &str = "d1.2";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiquidClassification {
    pub kind: ContentKind,
    pub contains_secret: bool,
}

fn map_choice_to_kind(choice: &str) -> Option<ContentKind> {
    match choice {
        "diagnostic" => Some(ContentKind::Diagnostic),
        "prose" => Some(ContentKind::Prose),
        "c_cpp" => Some(ContentKind::Code("cpp".to_string())),
        "rust" | "python" | "typescript" | "bash" | "json" | "go" | "sql" | "yaml" | "toml"
        | "css" | "html" => Some(ContentKind::Code(choice.to_string())),
        "other" => Some(ContentKind::Code(String::new())),
        _ => None,
    }
}

pub fn parse_liquid_response(body: &str) -> Option<LiquidClassification> {
    let parsed: Value = serde_json::from_str(body).ok()?;
    let answers = parsed.get("answers")?.as_object()?;
    let kind_obj = answers.get("content_kind")?.as_object()?;
    let choice = kind_obj.get("choice")?.as_str()?;
    let confidence = kind_obj
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    if confidence < 0.55 {
        return None;
    }
    let kind = map_choice_to_kind(choice)?;
    let contains_secret = answers
        .get("contains_secret")
        .and_then(|v| v.get("noul"))
        .and_then(|v| v.as_f64())
        .map(|p| p >= 0.75)
        .unwrap_or(false);

    Some(LiquidClassification {
        kind,
        contains_secret,
    })
}

pub fn build_request_body(text: &str) -> String {
    let clipped: String = text.chars().take(6000).collect();
    json!({
        "model": LIQUID_MODEL,
        "state": { "text": clipped },
        "questions": {
            "content_kind": {
                "type": "choice",
                "instructions": "What kind of content is `text`?",
                "criteria": {
                    "diagnostic": "Compiler error, stack trace, or failure log",
                    "rust": "Rust source code",
                    "python": "Python source code",
                    "typescript": "TypeScript or JavaScript source code",
                    "bash": "Shell commands or bash script",
                    "json": "JSON data",
                    "go": "Go source code",
                    "sql": "SQL query",
                    "yaml": "YAML config",
                    "toml": "TOML config",
                    "css": "CSS stylesheet",
                    "html": "HTML or JSX/TSX markup",
                    "c_cpp": "C or C++ source code",
                    "prose": "Natural language prose, documentation, or non-code text",
                    "other": "Code in another language"
                }
            },
            "contains_secret": {
                "type": "noul",
                "instructions": "Does `text` contain an API key, private token, password, or bearer credential?"
            }
        }
    })
    .to_string()
}

pub async fn classify_with_liquid(text: &str, api_key: &str) -> Option<LiquidClassification> {
    let key = api_key.trim();
    if key.is_empty() || text.trim().is_empty() {
        return None;
    }
    let payload = build_request_body(text);
    let output = Command::new("curl")
        .args([
            "-sS",
            "--max-time",
            "2",
            LIQUID_DECISIONS_URL,
            "-H",
            &format!("Authorization: Bearer {}", key),
            "-H",
            "Content-Type: application/json",
            "-d",
            &payload,
        ])
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }
    let body = String::from_utf8(output.stdout).ok()?;
    parse_liquid_response(&body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_liquid_classification_and_secret_flag() {
        let body = r#"{
            "answers": {
                "content_kind": { "type": "choice", "choice": "rust", "confidence": 0.92 },
                "contains_secret": { "type": "noul", "noul": 0.88 }
            }
        }"#;
        assert_eq!(
            parse_liquid_response(body),
            Some(LiquidClassification {
                kind: ContentKind::Code("rust".to_string()),
                contains_secret: true,
            })
        );
    }

    #[test]
    fn rejects_low_confidence_classification() {
        let body = r#"{
            "answers": {
                "content_kind": { "type": "choice", "choice": "python", "confidence": 0.31 },
                "contains_secret": { "type": "noul", "noul": 0.05 }
            }
        }"#;
        assert_eq!(parse_liquid_response(body), None);
    }
}
