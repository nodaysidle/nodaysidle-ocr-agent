# Release Notes: nodaysidle-ocr-agent v0.1.1

## v0.1.1 — Reliability, Safe Injection & Formatter Accuracy (2026-11-17)

Fixes every finding from the 2026-11-17 audit. No new dependencies, no API/LLM changes.

- **Failures are no longer silent (F01).** Every error path now fires a desktop notification (`omarchy-notification-send`, falling back to `notify-send -u critical`), not just success. "No text found in the selected region" gets its own message and exit code 2. Cancelling the selection still exits 0 quietly.
- **Typed errors with real tool stderr (F05, F16).** Errors are a structured `OcrError` enum instead of strings, and the last non-empty stderr line from `grim`/`tesseract` is carried into the message, so you see `Failed loading language 'slv'` instead of a generic failure.
- **The freeze survives capture (F03).** `hyprpicker` is now killed only after `grim` has the pixels, and the child is spawned with `kill_on_drop`, so a cancelled or failed `slurp` can never leave the screen frozen.
- **`--inject` can no longer hit the wrong pane (F02, F10, F11).** Exact case-insensitive match on the pane's command basename or the title's first token, focused pane only (no `tmux list-panes -a`), plain shells (`bash`, `zsh`, `fish`, `sh`, …) refused, text sent via `tmux load-buffer -` on stdin then `paste-buffer -p` (bracketed), and `--` before `herdr` text. The notification now reports the real outcome: injected, failed with a reason, or no pane found.
- **Language from the environment (F04).** `--lang` defaults to `OMARCHY_OCR_LANGS` (else `eng`), requests are validated against `tesseract --list-langs`, and a missing pack fails with `install tesseract-data-<lang>`.
- **Sharper OCR on HiDPI-thin text (F08).** Capture now uses `grim -s 2`, so 14 px screen glyphs reach tesseract at a readable size (`fn main()` instead of `©0 main()`).
- **Formatter no longer fences prose (F06).** Language detection is line-anchored and evidence-based, with JS/TS `import { … } from` checked before Python and diagnostics anchored, so ordinary sentences stay prose. Covered by the nine probe strings from the audit plus the existing cases.
- **Robustness (F12, F21).** A single-instance lock on `$XDG_RUNTIME_DIR/way-ocr-agent.lock` stops a double keypress from stacking two freeze overlays, and every subprocess (`grim`, `tesseract`, `herdr`, `tmux`, `wl-copy`) now has a timeout.
- **Packaging (F14, F15, F20).** Unused `serde`, `regex` and `tempfile` removed and `tokio` narrowed to `rt`, `macros`, `process`, `io-util`, `time`; CLI metadata read from Cargo; `tesseract-data-eng` moved to `depends`, `hyprpicker` to `optdepends`, `$CARCH` used in the binary PKGBUILD, and a real checksum replaces `SKIP`.
- **Docs corrected (F07, F09, F17, F19).** The false "preserves indentation depth" and sub-40 ms claims are gone, the word count excludes fence lines, the install instructions build the real binary name, and the Hyprland section documents `hl.unbind("SUPER + SHIFT + O")` plus a comparison with `omarchy-capture-text`.

---

## Initial Release for Wayland, Hyprland & Omarchy (2026-11-08)

`nodaysidle-ocr-agent` v0.1.0 is a fast, standalone screen-region OCR and code context injector built natively for Wayland compositors (Hyprland, Sway, Niri) and Omarchy Linux.

---

### The Problem Solved

Developers working alongside AI coding agents (Claude Code, Antigravity, OpenCode, Codex) frequently need to feed visual context to the model:
* Error traces in browser devtools or graphical consoles
* Code snippets in video presentations or documentation screenshots
* Text trapped in non-selectable canvas elements or remote VNCs

Traditional workarounds either require full-screen screenshots (which burn thousands of LLM vision tokens and add noise) or slow manual typing. `nodaysidle-ocr-agent` provides fast, in-RAM region capture (~250 ms OCR) that automatically reconstructs clean Markdown code blocks directly onto your Wayland clipboard.

---

### Key Capabilities in v0.1.0

- **Zero Disk I/O Pipeline:** Uses `hyprpicker` screen freezing + `slurp` crosshairs + `grim` pixel streaming piped directly into `tesseract` memory buffers in RAM. No temporary files are written to disk.
- **Smart Syntax & Diagnostic Reconstruction:**
  - Auto-detects programming languages (Rust, Python, TypeScript, Bash, JSON) and formats them into fenced code blocks (` ```rust `, ` ```python `).
  - Recognizes compiler diagnostics (`error[E...]`, stack traces, panics) and wraps them in clean diagnostic blocks (` ```text `).
  - Strips trailing whitespace and OCR scan artifacts.
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
