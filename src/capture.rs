use std::process::Stdio;
use tokio::process::Command;

pub struct ScreenCapture;

impl ScreenCapture {
    /// Freeze screen, prompt user for region with slurp, and stream PNG bytes from grim.
    pub async fn capture_region() -> Result<Vec<u8>, String> {
        // 1. Start hyprpicker freeze in background so screen does not shift during selection
        let mut picker = Command::new("hyprpicker")
            .args(["-r", "-z"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok();

        // Give hyprpicker 50ms to grab overlay
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // 2. Run slurp for interactive box selection
        let slurp_res = Command::new("slurp")
            .output()
            .await
            .map_err(|e| format!("Failed to run slurp: {}", e))?;

        // Teardown hyprpicker freeze
        if let Some(mut child) = picker.take() {
            let _ = child.kill().await;
        }

        if !slurp_res.status.success() {
            return Err("Selection cancelled".to_string());
        }

        let geometry = String::from_utf8_lossy(&slurp_res.stdout).trim().to_string();
        if geometry.is_empty() {
            return Err("Empty selection geometry".to_string());
        }

        // 3. Capture region via grim directly to stdout buffer (in RAM, no disk IO)
        let grim_output = Command::new("grim")
            .args(["-g", &geometry, "-"])
            .output()
            .await
            .map_err(|e| format!("Failed to run grim: {}", e))?;

        if !grim_output.status.success() {
            return Err("Failed to capture screen region with grim".to_string());
        }

        Ok(grim_output.stdout)
    }
}
