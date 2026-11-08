# Release Notes: nodaysidle-ocr-agent v0.1.0

## Initial Release for Wayland, Hyprland & Omarchy (2026-11-08)

`nodaysidle-ocr-agent` v0.1.0 is a fast, standalone screen-region OCR and code context injector built natively for Wayland compositors (Hyprland, Sway, Niri) and Omarchy Linux.

---

### The Problem Solved

Developers working alongside AI coding agents (Claude Code, Antigravity, OpenCode, Codex) frequently need to feed visual context to the model:
* Error traces in browser devtools or graphical consoles
* Code snippets in video presentations or documentation screenshots
* Text trapped in non-selectable canvas elements or remote VNCs

Traditional workarounds either require full-screen screenshots (which burn thousands of LLM vision tokens and add noise) or slow manual typing. `nodaysidle-ocr-agent` provides sub-40ms, in-RAM region capture that automatically reconstructs clean Markdown code blocks directly onto your Wayland clipboard.

---

### Key Capabilities in v0.1.0

- **Zero Disk I/O Pipeline:** Uses `hyprpicker` screen freezing + `slurp` crosshairs + `grim` pixel streaming piped directly into `tesseract` memory buffers in RAM. No temporary files are written to disk.
- **Smart Syntax & Diagnostic Reconstruction:**
  - Auto-detects programming languages (Rust, Python, TypeScript, Bash, JSON) and formats them into fenced code blocks (` ```rust `, ` ```python `).
  - Recognizes compiler diagnostics (`error[E...]`, stack traces, panics) and wraps them in clean diagnostic blocks (` ```text `).
  - Preserves indentation depth and strips OCR scan artifacts.
- **Clipboard-First Safety:** Always puts the formatted text onto your Wayland clipboard (`wl-copy`). It will never auto-paste into your active terminal or agent prompt unexpectedly. You paste with `Ctrl+Shift+V` or `Ctrl+V` whenever you choose.
- **Optional Background Agent Injection (`--inject`):** Can probe running terminal multiplexers ([Herdr](https://github.com/nodaysidle/herdr) or [Tmux](https://github.com/tmux/tmux)) and inject prompt text into the active agent pane without shifting desktop workspaces.
- **Desktop Feedback:** Sends a native desktop notification via `omarchy-notification-send` or `notify-send` showing captured word count.

---

### Quick Install (Precompiled Binary)

#### 1. Install System Dependencies (Arch / Omarchy)
```bash
sudo pacman -S --needed grim slurp hyprpicker tesseract tesseract-data-eng wl-clipboard
```

#### 2. Download and Install the Binary
```bash
curl -fsSL https://github.com/nodaysidle/nodaysidle-ocr-agent/releases/download/v0.1.0/nodaysidle-ocr-agent-v0.1.0-x86_64-unknown-linux-gnu.tar.gz | tar -xz -C ~/.local/bin/ nodaysidle-ocr-agent way-ocr-agent
chmod +x ~/.local/bin/nodaysidle-ocr-agent ~/.local/bin/way-ocr-agent
```

#### 3. Bind to `SUPER + SHIFT + O` in Hyprland
Add this line to your `~/.config/hypr/bindings.lua` (or `hyprland.conf`):

```lua
-- Wayland Fast Screen-Region OCR Context Injector
o.bind("SUPER + SHIFT + O", "Agent OCR Context Injector", "/home/arch/.local/bin/nodaysidle-ocr-agent")
```

Then reload Hyprland:
```bash
hyprctl reload
```

---

### Verification
```bash
nodaysidle-ocr-agent --help
nodaysidle-ocr-agent --version
```
Press `SUPER + SHIFT + O`, crop any text on screen, and paste (`Ctrl+V`) into your agent or editor!
