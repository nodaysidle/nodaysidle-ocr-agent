use nodaysidle_ocr_agent::formatter::{CodeFormatter, ContentKind};

#[test]
fn test_detect_rust_code() {
    let snippet = "pub fn calculate_sum(a: i32, b: i32) -> Result<i32, String> {
    let mut total = a + b;
    Ok(total)
}";
    let (formatted, kind) = CodeFormatter::format(snippet);
    assert!(matches!(kind, ContentKind::Code(lang) if lang == "rust"));
    assert!(formatted.starts_with(
        "```rust
"
    ));
    assert!(formatted.ends_with(
        "
```"
    ));
}

#[test]
fn test_detect_json() {
    let json = "{
  \"name\": \"way-ocr-agent\",
  \"version\": \"0.1.0\"
}";
    let (formatted, kind) = CodeFormatter::format(json);
    assert!(matches!(kind, ContentKind::Code(lang) if lang == "json"));
    assert!(formatted.starts_with(
        "```json
"
    ));
}

#[test]
fn test_detect_compiler_error() {
    let err = "error[E0433]: cannot find `scanner` in `crate`
  --> src/main.rs:10:5";
    let (formatted, kind) = CodeFormatter::format(err);
    assert_eq!(kind, ContentKind::Diagnostic);
    assert!(formatted.starts_with(
        "```text
"
    ));
}

#[test]
fn test_detect_plain_prose() {
    let prose =
        "This is a simple documentation sentence without any special syntax or code elements.";
    let (formatted, kind) = CodeFormatter::format(prose);
    assert_eq!(kind, ContentKind::Prose);
    assert_eq!(formatted, prose);
}

#[test]
fn test_formatter_probes_from_audit() {
    let prose_probes = [
        "Download the latest build from the release page.",
        "# Release notes
This week we shipped fixes.",
        "Let me know which match you prefer.",
        "Profit = revenue - costs; margin = profit / revenue; ratio <= 1; growth >= 0",
        "Undefined reference to the old spec in this chapter.",
        "Use git to clone the repo.",
        "Odprite datoteko in kliknite gumb Shrani.",
    ];
    for probe in prose_probes {
        let (formatted, kind) = CodeFormatter::format(probe);
        assert_eq!(kind, ContentKind::Prose, "Expected Prose for: {probe:?}");
        assert_eq!(formatted, probe);
    }

    let py_match = "def f(x):
    match x:
        case 1: return 0";
    let (_, py_kind) = CodeFormatter::format(py_match);
    assert_eq!(py_kind, ContentKind::Code("python".to_string()));

    let ts_import = "import { useState } from 'react';
const a = 1;";
    let (_, ts_kind) = CodeFormatter::format(ts_import);
    assert_eq!(ts_kind, ContentKind::Code("typescript".to_string()));
}

#[test]
fn test_detect_bash_and_python_snippets() {
    let (_, bash_kind) = CodeFormatter::format("$ git status && cargo test --offline");
    assert_eq!(bash_kind, ContentKind::Code("bash".to_string()));

    let (_, py_kind) = CodeFormatter::format(
        "from pathlib import Path
print(Path.cwd())",
    );
    assert_eq!(py_kind, ContentKind::Code("python".to_string()));
}

#[test]
fn test_detect_rust_println_and_linker_diagnostic() {
    let (_, rust_kind) = CodeFormatter::format("println!(\"hello\");");
    assert_eq!(rust_kind, ContentKind::Code("rust".to_string()));

    let (_, diag_kind) = CodeFormatter::format("main.o: undefined reference to `foo'");
    assert_eq!(diag_kind, ContentKind::Diagnostic);
}

#[test]
fn test_detect_empty_and_ts_arrow() {
    let (empty_out, empty_kind) = CodeFormatter::format("   ");
    assert_eq!(empty_kind, ContentKind::Prose);
    assert_eq!(empty_out, "");

    let (_, ts_kind) = CodeFormatter::format("const add = (a, b) => a + b;");
    assert_eq!(ts_kind, ContentKind::Code("typescript".to_string()));
}

#[test]
fn test_detect_bash_shebang() {
    let (_, kind) = CodeFormatter::format(
        "#!/usr/bin/env bash
echo ok",
    );
    assert_eq!(kind, ContentKind::Code("bash".to_string()));
}
