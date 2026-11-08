use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchTarget {
    ClipboardOnly,
    Herdr { pane_id: String, agent: String },
    Tmux { target: String, agent: String },
}

pub struct Dispatcher;

impl Dispatcher {
    pub const PREFERRED_AGENTS: &'static [&'static str] = &["claude", "agy", "codex", "pi", "hermes"];

    /// Always copy to Wayland clipboard using wl-copy.
    pub async fn copy_to_clipboard(text: &str) -> Result<(), String> {
        let mut child = Command::new("wl-copy")
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn wl-copy: {}", e))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes()).await
                .map_err(|e| format!("Failed to write to wl-copy: {}", e))?;
            drop(stdin);
        }

        let status = child.wait().await
            .map_err(|e| format!("Failed to wait for wl-copy: {}", e))?;

        if status.success() {
            Ok(())
        } else {
            Err("wl-copy exited with non-zero status".to_string())
        }
    }

    /// Probe for an active AI coding agent pane in Herdr or Tmux.
    pub async fn detect_agent_target() -> DispatchTarget {
        // 1. Probe Herdr
        if let Ok(output) = Command::new("herdr").args(["pane", "list"]).output().await {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                if let Some(panes) = json.pointer("/result/panes").and_then(|v| v.as_array()) {
                    for agent in Self::PREFERRED_AGENTS {
                        let agent_lower = agent.to_ascii_lowercase();
                        for p in panes {
                            let matches_agent = p.get("agent")
                                .and_then(|a| a.as_str())
                                .is_some_and(|a| a.to_ascii_lowercase() == agent_lower);

                            let matches_title = p.get("terminal_title")
                                .and_then(|t| t.as_str())
                                .is_some_and(|t| t.to_ascii_lowercase().contains(&agent_lower));

                            if matches_agent || matches_title {
                                if let Some(pane_id) = p.get("pane_id").and_then(|id| id.as_str()) {
                                    return DispatchTarget::Herdr {
                                        pane_id: pane_id.to_string(),
                                        agent: agent.to_string(),
                                    };
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Probe Tmux
        if let Ok(output) = Command::new("tmux")
            .args(["list-panes", "-a", "-F", "#{session_name}:#{window_index}.#{pane_index} #{pane_current_command}"])
            .output()
            .await
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let target = parts[0];
                    let cmd = parts[1].to_ascii_lowercase();
                    for agent in Self::PREFERRED_AGENTS {
                        if cmd.contains(&agent.to_ascii_lowercase()) {
                            return DispatchTarget::Tmux {
                                target: target.to_string(),
                                agent: agent.to_string(),
                            };
                        }
                    }
                }
            }
        }

        DispatchTarget::ClipboardOnly
    }

    /// Inject prompt text into active agent pane.
    pub async fn inject_to_agent(target: &DispatchTarget, text: &str) -> Result<(), String> {
        match target {
            DispatchTarget::Herdr { pane_id, .. } => {
                let status = Command::new("herdr")
                    .args(["pane", "send-text", pane_id, text])
                    .status()
                    .await
                    .map_err(|e| format!("Failed to send text to Herdr pane: {}", e))?;

                if status.success() {
                    Ok(())
                } else {
                    Err("Herdr send-text exited with failure".to_string())
                }
            }
            DispatchTarget::Tmux { target, .. } => {
                let status = Command::new("tmux")
                    .args(["set-buffer", text])
                    .status()
                    .await
                    .map_err(|e| e.to_string())?;

                if status.success() {
                    let _ = Command::new("tmux").args(["paste-buffer", "-t", target]).status().await;
                    Ok(())
                } else {
                    Err("Tmux paste failed".to_string())
                }
            }
            DispatchTarget::ClipboardOnly => Ok(()),
        }
    }

    /// Send desktop notification with capture stats.
    pub async fn notify(word_count: usize, target: &DispatchTarget) {
        let msg = match target {
            DispatchTarget::Herdr { agent, .. } => {
                format!("{} words captured -> Copied to clipboard & injected to {}", word_count, agent)
            }
            DispatchTarget::Tmux { agent, .. } => {
                format!("{} words captured -> Copied to clipboard & injected to {}", word_count, agent)
            }
            DispatchTarget::ClipboardOnly => {
                format!("{} words captured -> Copied to clipboard (ready to paste)", word_count)
            }
        };

        if Command::new("omarchy-notification-send")
            .args(["-g", "󰴑", &msg])
            .status()
            .await
            .is_err()
        {
            let _ = Command::new("notify-send")
                .args(["way-ocr-agent", &msg])
                .status()
                .await;
        }
    }
}
