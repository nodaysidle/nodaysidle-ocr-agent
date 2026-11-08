# way-ocr-agent

Fast Wayland Screen-Region OCR and Code Context Injector for AI coding agents (Claude Code, Antigravity / AGY, Codex, Hermes, Pi) and terminal multiplexers (Herdr, Tmux).

## Why It Exists

Standard screenshot tools burn thousands of image tokens and clutter context windows. `way-ocr-agent` lets you select an arbitrary screen region on Wayland (Hyprland), executes OCR locally in RAM in ~30ms, automatically formats programming languages into markdown code fences, and injects the text directly into your active coding agent or clipboard.

---

## Features
* **Zero Disk I/O**: `hyprpicker -r -z` frozen overlay + `slurp` crosshair + `grim` piped directly into `tesseract` memory buffers.
* **Code & Error Awareness**: Automatically recognizes code structures (Rust, Python, TypeScript, Bash, JSON) or compiler diagnostics/stack traces and wraps them in clean markdown fences (` ```rust `, ` ```text `).
* **Two-Pass Agent Injection**: Automatically discovers running agent panes in Herdr or Tmux and injects the prompt without stealing window focus.
* **Wayland Clipboard Fallback**: Automatically copies the clean markdown snippet to `wl-copy`.

---

## Usage
```bash
# Capture region and inject to active agent or clipboard
way-ocr-agent

# Capture and print directly to stdout
way-ocr-agent --stdout

# Clipboard only (skip agent pane injection)
way-ocr-agent --no-inject

# Output raw unformatted OCR text
way-ocr-agent --raw
```

## Keybinding
Bound to `SUPER + SHIFT + O` in `~/.config/hypr/bindings.lua`.
