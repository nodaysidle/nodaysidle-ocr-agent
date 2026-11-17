use std::fmt;
use std::future::Future;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OcrError {
    Cancelled,
    MissingTool(&'static str),
    MissingLang {
        lang: String,
    },
    NoText,
    EmptyGeometry,
    Tool {
        name: &'static str,
        stderr: String,
    },
    Timeout {
        name: &'static str,
        seconds: u64,
    },
    Io {
        context: &'static str,
        detail: String,
    },
}

impl OcrError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Cancelled => 0,
            Self::NoText => 2,
            _ => 1,
        }
    }

    pub fn notification_message(&self) -> String {
        match self {
            Self::Cancelled => "Selection cancelled".to_string(),
            Self::MissingTool(tool) => format!("Missing required tool: {tool}"),
            Self::MissingLang { lang } => {
                format!("Tesseract language '{lang}' not installed (install tesseract-data-{lang})")
            }
            Self::NoText => "No text found in the selected region".to_string(),
            Self::EmptyGeometry => "Empty selection geometry".to_string(),
            Self::Tool { name, stderr } => {
                if stderr.is_empty() {
                    format!("{name} failed")
                } else {
                    format!("{name} failed: {stderr}")
                }
            }
            Self::Timeout { name, seconds } => {
                format!("{name} timed out after {seconds}s")
            }
            Self::Io { context, detail } => format!("{context}: {detail}"),
        }
    }
}

impl fmt::Display for OcrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.notification_message())
    }
}

impl std::error::Error for OcrError {}

pub fn last_non_empty_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .to_string()
}

pub async fn with_timeout<T, E, F>(name: &'static str, seconds: u64, fut: F) -> Result<T, OcrError>
where
    F: Future<Output = Result<T, E>>,
    E: Into<OcrError>,
{
    match tokio::time::timeout(Duration::from_secs(seconds), fut).await {
        Ok(res) => res.map_err(Into::into),
        Err(_) => Err(OcrError::Timeout { name, seconds }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_messages_and_exit_codes() {
        assert_eq!(OcrError::Cancelled.exit_code(), 0);
        assert_eq!(
            OcrError::Cancelled.notification_message(),
            "Selection cancelled"
        );

        assert_eq!(OcrError::NoText.exit_code(), 2);
        assert_eq!(
            OcrError::NoText.notification_message(),
            "No text found in the selected region"
        );

        let missing_tool = OcrError::MissingTool("grim");
        assert_eq!(missing_tool.exit_code(), 1);
        assert_eq!(
            missing_tool.notification_message(),
            "Missing required tool: grim"
        );

        let missing_lang = OcrError::MissingLang {
            lang: "slv".to_string(),
        };
        assert_eq!(missing_lang.exit_code(), 1);
        assert_eq!(
            missing_lang.notification_message(),
            "Tesseract language 'slv' not installed (install tesseract-data-slv)"
        );

        let empty_geom = OcrError::EmptyGeometry;
        assert_eq!(empty_geom.exit_code(), 1);
        assert_eq!(
            empty_geom.notification_message(),
            "Empty selection geometry"
        );

        let tool_err = OcrError::Tool {
            name: "tesseract",
            stderr: "Failed loading language 'slv'".to_string(),
        };
        assert_eq!(tool_err.exit_code(), 1);
        assert_eq!(
            tool_err.notification_message(),
            "tesseract failed: Failed loading language 'slv'"
        );

        let timeout_err = OcrError::Timeout {
            name: "tesseract",
            seconds: 20,
        };
        assert_eq!(timeout_err.exit_code(), 1);
        assert_eq!(
            timeout_err.notification_message(),
            "tesseract timed out after 20s"
        );

        let io_err = OcrError::Io {
            context: "Failed to run slurp",
            detail: "permission denied".to_string(),
        };
        assert_eq!(io_err.exit_code(), 1);
        assert_eq!(
            io_err.notification_message(),
            "Failed to run slurp: permission denied"
        );
    }
}
