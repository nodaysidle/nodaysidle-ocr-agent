use nodaysidle_ocr_agent::formatter::{CodeFormatter, ContentKind};

#[test]
fn test_detect_rust_code() {
    let snippet = "pub fn calculate_sum(a: i32, b: i32) -> Result<i32, String> {\n    let mut total = a + b;\n    Ok(total)\n}";
    let (formatted, kind) = CodeFormatter::format(snippet);
    assert!(matches!(kind, ContentKind::Code(lang) if lang == "rust"));
    assert!(formatted.starts_with("```rust\n"));
    assert!(formatted.ends_with("\n```"));
}

#[test]
fn test_detect_json() {
    let json = "{\n  \"name\": \"way-ocr-agent\",\n  \"version\": \"0.1.0\"\n}";
    let (formatted, kind) = CodeFormatter::format(json);
    assert!(matches!(kind, ContentKind::Code(lang) if lang == "json"));
    assert!(formatted.starts_with("```json\n"));
}

#[test]
fn test_detect_compiler_error() {
    let err = "error[E0433]: cannot find `scanner` in `crate`\n  --> src/main.rs:10:5";
    let (formatted, kind) = CodeFormatter::format(err);
    assert_eq!(kind, ContentKind::Diagnostic);
    assert!(formatted.starts_with("```text\n"));
}

#[test]
fn test_detect_plain_prose() {
    let prose = "This is a simple documentation sentence without any special syntax or code elements.";
    let (formatted, kind) = CodeFormatter::format(prose);
    assert_eq!(kind, ContentKind::Prose);
    assert_eq!(formatted, prose);
}
