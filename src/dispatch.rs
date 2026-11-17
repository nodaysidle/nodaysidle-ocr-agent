use crate::error::{last_non_empty_line, with_timeout, OcrError};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchTarget {
    ClipboardOnly,
    Herdr { pane_id: String, agent: String },
    Tmux { target: String, agent: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InjectOutcome {
    Skipped,
    Injected { agent: String },
    NoTarget,
    Failed { reason: String },
}

pub struct Dispatcher;

const PLAIN_SHELLS: &[&str] = &["bash", "zsh", "fish", "sh", "dash", "nu", "elvish", "pwsh"];

impl Dispatcher {
    pub const PREFERRED_AGENTS: &'static [&'static str] =
        &["claude", "agy", "codex", "pi", "hermes"];

    /// Returns true if the command basename refers to a plain interactive shell.
    pub fn is_plain_shell(command: &str) -> bool {
        let token = command.split_whitespace().next().unwrap_or("");
        let basename = token.rsplit('/').next().unwrap_or(token);
        PLAIN_SHELLS
            .iter()
            .any(|shell| basename.eq_ignore_ascii_case(shell))
    }

    /// Exact, case-insensitive match on command basename or title's first token, rejecting plain shells.
    pub fn matches_agent(candidate: &str, agent: &str) -> bool {
        let token = candidate.split_whitespace().next().unwrap_or("");
        let basename = token.rsplit('/').next().unwrap_or(token);
        if basename.is_empty() || Self::is_plain_shell(basename) {
            return false;
        }
        basename.eq_ignore_ascii_case(agent)
    }

    /// Build the user-facing notification message from word count and injection outcome.
    pub fn build_notification_message(word_count: usize, outcome: &InjectOutcome) -> String {
        match outcome {
            InjectOutcome::Skipped => {
                format!("{word_count} words captured -> Copied to clipboard (ready to paste)")
            }
            InjectOutcome::Injected { agent } => {
                format!("{word_count} words captured -> Copied to clipboard & injected to {agent}")
            }
            InjectOutcome::NoTarget => {
                format!("{word_count} words captured -> no agent pane found, clipboard only")
            }
            InjectOutcome::Failed { reason } => {
                format!(
                    "{word_count} words captured -> Copied to clipboard, inject failed: {reason}"
                )
            }
        }
    }

    /// Always copy to Wayland clipboard using wl-copy.
    pub async fn copy_to_clipboard(text: &str) -> Result<(), OcrError> {
        let mut child = Command::new("wl-copy")
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    OcrError::MissingTool("wl-copy")
                } else {
                    OcrError::Io {
                        context: "Failed to spawn wl-copy",
                        detail: e.to_string(),
                    }
                }
            })?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(text.as_bytes())
                .await
                .map_err(|e| OcrError::Io {
                    context: "Failed to write to wl-copy",
                    detail: e.to_string(),
                })?;
            drop(stdin);
        }

        let output = with_timeout("wl-copy", 10, async {
            child.wait_with_output().await.map_err(|e| OcrError::Io {
                context: "Failed to wait for wl-copy",
                detail: e.to_string(),
            })
        })
        .await?;

        if output.status.success() {
            Ok(())
        } else {
            Err(OcrError::Tool {
                name: "wl-copy",
                stderr: last_non_empty_line(&output.stderr),
            })
        }
    }

    fn match_herdr_pane(pane: &serde_json::Value) -> Option<DispatchTarget> {
        let pane_id = pane.get("pane_id").and_then(|id| id.as_str())?;
        let title = pane
            .get("terminal_title")
            .and_then(|t| t.as_str())
            .unwrap_or_default();
        let cmd = pane
            .get("command")
            .and_then(|c| c.as_str())
            .unwrap_or(title);

        if Self::is_plain_shell(cmd) {
            return None;
        }

        for agent in Self::PREFERRED_AGENTS {
            if Self::matches_agent(cmd, agent) || Self::matches_agent(title, agent) {
                return Some(DispatchTarget::Herdr {
                    pane_id: pane_id.to_string(),
                    agent: (*agent).to_string(),
                });
            }
        }
        None
    }

    /// Probe for an active, focused AI coding agent pane in Herdr or Tmux.
    pub async fn detect_agent_target() -> DispatchTarget {
        // 1. Probe focused Herdr pane (`herdr pane current`, then focused entry in `herdr pane list`)
        if let Ok(Ok(output)) = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            Command::new("herdr").args(["pane", "current"]).output(),
        )
        .await
        {
            if output.status.success() {
                if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                    let pane = json
                        .pointer("/result/pane")
                        .or_else(|| json.pointer("/result"));
                    if let Some(p) = pane {
                        if let Some(target) = Self::match_herdr_pane(p) {
                            return target;
                        }
                    }
                }
            }
        }

        if let Ok(Ok(output)) = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            Command::new("herdr").args(["pane", "list"]).output(),
        )
        .await
        {
            if output.status.success() {
                if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                    if let Some(panes) = json.pointer("/result/panes").and_then(|v| v.as_array()) {
                        for p in panes {
                            if p.get("focused").and_then(|v| v.as_bool()) == Some(true) {
                                if let Some(target) = Self::match_herdr_pane(p) {
                                    return target;
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Probe focused Tmux pane in attached session only (no `list-panes -a`)
        if let Ok(Ok(output)) = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            Command::new("tmux")
                .args([
                    "list-panes",
                    "-F",
                    "#{session_name}:#{window_index}.#{pane_index} #{pane_active} #{pane_current_command}",
                ])
                .output(),
        )
        .await
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 && parts[1] == "1" {
                        let target = parts[0];
                        let cmd = parts[2];
                        for agent in Self::PREFERRED_AGENTS {
                            if Self::matches_agent(cmd, agent) {
                                return DispatchTarget::Tmux {
                                    target: target.to_string(),
                                    agent: (*agent).to_string(),
                                };
                            }
                        }
                    }
                }
            }
        }

        DispatchTarget::ClipboardOnly
    }

    /// Inject prompt text into active agent pane and return honest outcome.
    pub async fn inject_to_agent(target: &DispatchTarget, text: &str) -> InjectOutcome {
        match target {
            DispatchTarget::Herdr { pane_id, agent } => {
                match with_timeout("herdr", 10, async {
                    Command::new("herdr")
                        .args(["pane", "send-text", pane_id, "--", text])
                        .output()
                        .await
                        .map_err(|e| OcrError::Io {
                            context: "Failed to run herdr",
                            detail: e.to_string(),
                        })
                })
                .await
                {
                    Ok(output) if output.status.success() => InjectOutcome::Injected {
                        agent: agent.clone(),
                    },
                    Ok(output) => {
                        let reason = last_non_empty_line(&output.stderr);
                        InjectOutcome::Failed {
                            reason: if reason.is_empty() {
                                "herdr send-text failed".to_string()
                            } else {
                                reason
                            },
                        }
                    }
                    Err(e) => InjectOutcome::Failed {
                        reason: e.notification_message(),
                    },
                }
            }
            DispatchTarget::Tmux { target, agent } => {
                let mut child = match Command::new("tmux")
                    .args(["load-buffer", "-"])
                    .stdin(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                {
                    Ok(child) => child,
                    Err(e) => {
                        return InjectOutcome::Failed {
                            reason: e.to_string(),
                        };
                    }
                };

                if let Some(mut stdin) = child.stdin.take() {
                    if let Err(e) = stdin.write_all(text.as_bytes()).await {
                        return InjectOutcome::Failed {
                            reason: e.to_string(),
                        };
                    }
                    drop(stdin);
                }

                match with_timeout("tmux", 10, async {
                    child.wait_with_output().await.map_err(|e| OcrError::Io {
                        context: "Failed to wait for tmux load-buffer",
                        detail: e.to_string(),
                    })
                })
                .await
                {
                    Ok(output) if output.status.success() => {} // proceed to paste-buffer
                    Ok(output) => {
                        let reason = last_non_empty_line(&output.stderr);
                        return InjectOutcome::Failed {
                            reason: if reason.is_empty() {
                                "tmux load-buffer failed".to_string()
                            } else {
                                reason
                            },
                        };
                    }
                    Err(e) => {
                        return InjectOutcome::Failed {
                            reason: e.notification_message(),
                        };
                    }
                }

                match with_timeout("tmux", 10, async {
                    Command::new("tmux")
                        .args(["paste-buffer", "-p", "-t", target])
                        .output()
                        .await
                        .map_err(|e| OcrError::Io {
                            context: "Failed to run tmux paste-buffer",
                            detail: e.to_string(),
                        })
                })
                .await
                {
                    Ok(output) if output.status.success() => InjectOutcome::Injected {
                        agent: agent.clone(),
                    },
                    Ok(output) => {
                        let reason = last_non_empty_line(&output.stderr);
                        InjectOutcome::Failed {
                            reason: if reason.is_empty() {
                                "tmux paste-buffer failed".to_string()
                            } else {
                                reason
                            },
                        }
                    }
                    Err(e) => InjectOutcome::Failed {
                        reason: e.notification_message(),
                    },
                }
            }
            DispatchTarget::ClipboardOnly => InjectOutcome::NoTarget,
        }
    }

    /// Send desktop notification with capture stats and honest injection outcome.
    pub async fn notify(word_count: usize, outcome: &InjectOutcome) {
        let msg = Self::build_notification_message(word_count, outcome);
        if matches!(outcome, InjectOutcome::Failed { .. }) {
            Self::notify_error(&msg).await;
            return;
        }

        if Command::new("omarchy-notification-send")
            .args(["-g", "", &msg])
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

    /// Send a critical desktop notification on failure.
    pub async fn notify_error(msg: &str) {
        if Command::new("omarchy-notification-send")
            .args(["-g", "", msg])
            .status()
            .await
            .is_err()
        {
            let _ = Command::new("notify-send")
                .args(["-u", "critical", "way-ocr-agent", msg])
                .status()
                .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_agent_exact_and_rejects_substrings_and_shells() {
        assert!(!Dispatcher::matches_agent("pipx", "pi"));
        assert!(!Dispatcher::matches_agent("spinner", "pi"));
        assert!(!Dispatcher::matches_agent("~/pictures", "pi"));
        assert!(Dispatcher::matches_agent("pi", "pi"));
        assert!(Dispatcher::matches_agent("/usr/bin/PI", "pi"));
        assert!(Dispatcher::matches_agent("hermes --tui", "hermes"));
        assert!(!Dispatcher::matches_agent("bash", "bash"));
        assert!(!Dispatcher::matches_agent("zsh", "pi"));
        assert!(!Dispatcher::matches_agent("fish", "codex"));
        assert!(Dispatcher::is_plain_shell("sh"));
        assert!(Dispatcher::is_plain_shell("/bin/fish"));
    }

    #[test]
    fn test_notification_messages_for_all_outcomes() {
        assert_eq!(
            Dispatcher::build_notification_message(
                12,
                &InjectOutcome::Injected {
                    agent: "claude".to_string()
                }
            ),
            "12 words captured -> Copied to clipboard & injected to claude"
        );
        assert_eq!(
            Dispatcher::build_notification_message(
                12,
                &InjectOutcome::Failed {
                    reason: "pane closed".to_string()
                }
            ),
            "12 words captured -> Copied to clipboard, inject failed: pane closed"
        );
        assert_eq!(
            Dispatcher::build_notification_message(12, &InjectOutcome::NoTarget),
            "12 words captured -> no agent pane found, clipboard only"
        );
        assert_eq!(
            Dispatcher::build_notification_message(12, &InjectOutcome::Skipped),
            "12 words captured -> Copied to clipboard (ready to paste)"
        );
    }
}
