use crate::error::{last_non_empty_line, with_timeout, OcrError};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub struct OcrEngine;

impl OcrEngine {
    /// Verify all requested `+`-separated languages are present in `tesseract --list-langs` output.
    pub fn validate_langs(requested: &str, list_langs_stdout: &str) -> Result<(), OcrError> {
        let available: Vec<&str> = list_langs_stdout
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with("List of available languages"))
            .collect();

        for part in requested
            .split('+')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            if !available.contains(&part) {
                return Err(OcrError::MissingLang {
                    lang: part.to_string(),
                });
            }
        }
        Ok(())
    }

    /// Check installed tesseract languages before running OCR.
    pub async fn check_langs(requested: &str) -> Result<(), OcrError> {
        let output = with_timeout("tesseract", 10, async {
            Command::new("tesseract")
                .arg("--list-langs")
                .output()
                .await
                .map_err(|e| {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        OcrError::MissingTool("tesseract")
                    } else {
                        OcrError::Io {
                            context: "Failed to run tesseract --list-langs",
                            detail: e.to_string(),
                        }
                    }
                })
        })
        .await?;

        if !output.status.success() {
            return Err(OcrError::Tool {
                name: "tesseract",
                stderr: last_non_empty_line(&output.stderr),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::validate_langs(requested, &stdout)
    }

    /// Pipe raw image bytes into tesseract and return extracted text.
    pub async fn extract_text(image_bytes: &[u8], lang: &str) -> Result<String, OcrError> {
        Self::check_langs(lang).await?;
        let mut child = Command::new("tesseract")
            .args([
                "stdin",
                "stdout",
                "--oem",
                "1",
                "--psm",
                "6",
                "-l",
                lang,
                "--dpi",
                "300",
                "-c",
                "preserve_interword_spaces=1",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    OcrError::MissingTool("tesseract")
                } else {
                    OcrError::Io {
                        context: "Failed to spawn tesseract",
                        detail: e.to_string(),
                    }
                }
            })?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(image_bytes)
                .await
                .map_err(|e| OcrError::Io {
                    context: "Failed to write image to tesseract stdin",
                    detail: e.to_string(),
                })?;
            drop(stdin);
        }

        let output = with_timeout("tesseract", 20, async {
            child.wait_with_output().await.map_err(|e| OcrError::Io {
                context: "Failed to wait for tesseract",
                detail: e.to_string(),
            })
        })
        .await?;

        if !output.status.success() {
            return Err(OcrError::Tool {
                name: "tesseract",
                stderr: last_non_empty_line(&output.stderr),
            });
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() {
            return Err(OcrError::NoText);
        }

        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_langs_missing_gives_install_hint() {
        let sample = "List of available languages in \"/usr/share/tessdata/\" (2):\neng\nosd\n";
        assert_eq!(OcrEngine::validate_langs("eng", sample), Ok(()));
        let err = OcrEngine::validate_langs("eng+slv", sample).unwrap_err();
        assert_eq!(
            err.notification_message(),
            "Tesseract language 'slv' not installed (install tesseract-data-slv)"
        );
    }
}
