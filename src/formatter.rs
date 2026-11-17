#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentKind {
    Code(String), // Language hint
    Diagnostic,   // Error/stack trace
    Prose,        // Natural language
}

pub struct CodeFormatter;

impl CodeFormatter {
    pub fn format(raw_text: &str) -> (String, ContentKind) {
        let cleaned = Self::clean_whitespace(raw_text);
        let kind = Self::detect_kind(&cleaned);

        match &kind {
            ContentKind::Code(lang) => {
                let formatted = format!(
                    "```{}
{}
```",
                    lang, cleaned
                );
                (formatted, kind)
            }
            ContentKind::Diagnostic => {
                let formatted = format!(
                    "```text
{}
```",
                    cleaned
                );
                (formatted, kind)
            }
            ContentKind::Prose => (cleaned, kind),
        }
    }

    fn clean_whitespace(text: &str) -> String {
        let lines: Vec<&str> = text.lines().map(|l| l.trim_end()).collect();
        lines.join(
            "
",
        )
    }

    fn has_line_prefix(lines: &[&str], prefixes: &[&str]) -> bool {
        lines.iter().any(|l| {
            let t = l.trim_start();
            prefixes.iter().any(|p| t.starts_with(p))
        })
    }

    pub fn detect_kind(text: &str) -> ContentKind {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return ContentKind::Prose;
        }
        let lines: Vec<&str> = trimmed.lines().collect();

        // 1. Check for compiler diagnostics or stack traces (anchored)
        if lines.iter().any(|l| {
            let t = l.trim_start();
            t.contains("error[E")
                || t.contains("Traceback (most recent call last):")
                || t.contains("Exception in thread")
                || t.contains("FAILED (")
                || t.starts_with("FATAL:")
                || t.starts_with("panic:")
                || t.contains("panicked at")
                || t.contains("SIGSEGV")
                || t.contains("undefined reference to `")
                || t.contains("Undefined reference to `")
        }) {
            return ContentKind::Diagnostic;
        }

        // 2. Check for JSON
        if ((trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']')))
            && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
        {
            return ContentKind::Code("json".to_string());
        }

        // 3. Check for TypeScript / JavaScript before Python (handles `import { ... } from '...'`)
        let has_js_import =
            Self::has_line_prefix(&lines, &["import {", "import * as ", "import type "])
                && trimmed.contains(" from ");
        let has_js_decl =
            Self::has_line_prefix(
                &lines,
                &["const ", "function ", "export ", "interface ", "type "],
            ) && (trimmed.contains('=') || trimmed.contains('{') || trimmed.contains('('));
        if has_js_import
            || has_js_decl
            || trimmed.contains("console.log(")
            || (trimmed.contains("=>") && (trimmed.contains('(') || trimmed.contains('{')))
        {
            return ContentKind::Code("typescript".to_string());
        }

        // 4. Check for Python before Rust (handles Python 3.10+ `match`/`case` inside `def`)
        let has_py_def = Self::has_line_prefix(&lines, &["def ", "async def ", "class "])
            && trimmed.contains('(')
            && trimmed.contains(':');
        let has_py_import = (Self::has_line_prefix(&lines, &["import "])
            && (lines.len() >= 2 || trimmed.contains(" as ")))
            || (Self::has_line_prefix(&lines, &["from "]) && trimmed.contains(" import "));
        let has_py_ctrl = Self::has_line_prefix(&lines, &["elif "]) && trimmed.contains(':');
        if has_py_def || has_py_import || has_py_ctrl || trimmed.contains("__init__") {
            return ContentKind::Code("python".to_string());
        }

        // 5. Check for Rust indicators
        let has_rust_decl = Self::has_line_prefix(
            &lines,
            &[
                "fn ",
                "pub fn ",
                "async fn ",
                "pub async fn ",
                "impl ",
                "struct ",
                "pub struct ",
                "enum ",
                "pub enum ",
                "let mut ",
            ],
        );
        let has_rust_sym = trimmed.contains("->")
            || trimmed.contains("::")
            || trimmed.contains("Result<")
            || trimmed.contains("Option<")
            || trimmed.contains("println!(");
        if (has_rust_decl && (has_rust_sym || (trimmed.contains('{') && trimmed.contains('}'))))
            || trimmed.contains("println!(")
        {
            return ContentKind::Code("rust".to_string());
        }

        // 6. Check for Shell commands (anchored)
        let has_shell_prompt = Self::has_line_prefix(&lines, &["$ ", "#!/"]);
        let has_shell_cmd =
            Self::has_line_prefix(&lines, &["sudo ", "cargo ", "pacman ", "npm ", "git "])
                && (lines.len() >= 2
                    || trimmed.contains(" && ")
                    || trimmed.contains(" | ")
                    || trimmed.contains("--"));
        if has_shell_prompt || has_shell_cmd {
            return ContentKind::Code("bash".to_string());
        }

        // Fallback: structural code density
        let structural_chars = trimmed
            .chars()
            .filter(|&c| c == '{' || c == '}' || c == ';')
            .count();
        if structural_chars >= 2 && lines.len() >= 2 {
            return ContentKind::Code("text".to_string());
        }

        ContentKind::Prose
    }
}
