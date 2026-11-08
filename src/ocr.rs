use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub struct OcrEngine;

impl OcrEngine {
    /// Pipe raw image bytes into tesseract and return extracted text.
    pub async fn extract_text(image_bytes: &[u8], lang: &str) -> Result<String, String> {
        let mut child = Command::new("tesseract")
            .args([
                "stdin",
                "stdout",
                "--oem", "1",
                "--psm", "6",
                "-l", lang,
                "--dpi", "300",
                "-c", "preserve_interword_spaces=1",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn tesseract: {}", e))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(image_bytes).await
                .map_err(|e| format!("Failed to write image to tesseract stdin: {}", e))?;
            drop(stdin);
        }

        let output = child.wait_with_output().await
            .map_err(|e| format!("Failed to wait for tesseract: {}", e))?;

        if !output.status.success() {
            return Err("Tesseract failed to extract text".to_string());
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() {
            return Err("No text detected in selected region".to_string());
        }

        Ok(text)
    }
}
