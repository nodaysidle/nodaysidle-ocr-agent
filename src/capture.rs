use crate::error::{last_non_empty_line, with_timeout, OcrError};
use std::process::Stdio;
use tokio::process::Command;

pub struct ScreenCapture;

impl ScreenCapture {
    /// Freeze screen, prompt user for region with slurp, and stream PNG bytes from grim.
    pub async fn capture_region() -> Result<Vec<u8>, OcrError> {
        // 1. Start hyprpicker freeze in background so screen does not shift during selection
        let mut picker = match Command::new("hyprpicker")
            .args(["-r", "-z"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(child) => {
                // Give hyprpicker 100ms to grab overlay
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                Some(child)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("warning: hyprpicker not found; capturing without screen freeze");
                None
            }
            Err(e) => {
                eprintln!("warning: failed to spawn hyprpicker: {e}");
                None
            }
        };

        // 2. Run slurp for interactive box selection
        let slurp_res = Command::new("slurp").output().await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                OcrError::MissingTool("slurp")
            } else {
                OcrError::Io {
                    context: "Failed to run slurp",
                    detail: e.to_string(),
                }
            }
        })?;

        if !slurp_res.status.success() {
            if let Some(mut child) = picker.take() {
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
            return Err(OcrError::Cancelled);
        }

        let geometry = String::from_utf8_lossy(&slurp_res.stdout)
            .trim()
            .to_string();
        if geometry.is_empty() {
            if let Some(mut child) = picker.take() {
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
            return Err(OcrError::EmptyGeometry);
        }

        // 3. Capture region via grim at 2x scale directly to stdout buffer (in RAM, no disk IO)
        let grim_res = with_timeout("grim", 15, async {
            Command::new("grim")
                .args(["-s", "2", "-g", &geometry, "-"])
                .output()
                .await
                .map_err(|e| {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        OcrError::MissingTool("grim")
                    } else {
                        OcrError::Io {
                            context: "Failed to run grim",
                            detail: e.to_string(),
                        }
                    }
                })
        })
        .await;

        // Teardown hyprpicker freeze only after grim finishes
        if let Some(mut child) = picker.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }

        let grim_output = grim_res?;

        if !grim_output.status.success() {
            return Err(OcrError::Tool {
                name: "grim",
                stderr: last_non_empty_line(&grim_output.stderr),
            });
        }

        Ok(grim_output.stdout)
    }
}
