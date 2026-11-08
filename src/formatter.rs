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
                let formatted = format!("```{}\n{}\n```", lang, cleaned);
                (formatted, kind)
            }
            ContentKind::Diagnostic => {
                let formatted = format!("```text\n{}\n```", cleaned);
                (formatted, kind)
            }
            ContentKind::Prose => (cleaned, kind),
        }
    }

    fn clean_whitespace(text: &str) -> String {
        let lines: Vec<&str> = text.lines().map(|l| l.trim_end()).collect();
        lines.join("\n")
    }

    pub fn detect_kind(text: &str) -> ContentKind {
        let lines: Vec<&str> = text.lines().collect();

        // 1. Check for compiler diagnostics or stack traces
        if text.contains("error[E")
            || text.contains("Traceback (most recent call last):")
            || text.contains("Exception in thread")
            || text.contains("FAILED (")
            || text.contains("FATAL:")
            || text.contains("panic:")
            || text.contains("Undefined reference to")
            || text.contains("SIGSEGV")
        {
            return ContentKind::Diagnostic;
        }

        // 2. Check for JSON
        let trimmed = text.trim();
        if (trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        {
            if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
                return ContentKind::Code("json".to_string());
            }
        }

        // 3. Check for Rust indicators
        if text.contains("fn ")
            || text.contains("let mut ")
            || text.contains("impl ")
            || text.contains("pub struct ")
            || text.contains("println!")
            || text.contains("Result<")
            || text.contains("Option<")
            || text.contains("match ")
        {
            return ContentKind::Code("rust".to_string());
        }

        // 4. Check for Python indicators
        if text.contains("def ")
            || text.contains("import ")
            || text.contains("from ")
            || text.contains("class ")
            || text.contains("print(")
            || text.contains("__init__")
            || text.contains("elif ")
        {
            return ContentKind::Code("python".to_string());
        }

        // 5. Check for TypeScript / JavaScript
        if text.contains("const ")
            || text.contains("function ")
            || text.contains("export ")
            || text.contains("interface ")
            || text.contains("console.log")
            || text.contains("=>")
        {
            return ContentKind::Code("typescript".to_string());
        }

        // 6. Check for Shell commands
        if lines.iter().any(|l| l.trim_start().starts_with('$') || l.trim_start().starts_with('#'))
            || text.contains("sudo ")
            || text.contains("cargo ")
            || text.contains("pacman ")
            || text.contains("npm ")
            || text.contains("git ")
        {
            return ContentKind::Code("bash".to_string());
        }

        // Fallback: If it has programming symbols (semicolons, braces, arrows, camelCase)
        let code_chars = text.chars().filter(|&c| c == '{' || c == '}' || c == ';' || c == '=' || c == '<' || c == '>').count();
        if code_chars >= 4 && lines.len() >= 2 {
            return ContentKind::Code("text".to_string());
        }

        ContentKind::Prose
    }
}
