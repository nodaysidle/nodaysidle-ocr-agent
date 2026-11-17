# HANDOFF — nodaysidle-ocr-agent

Date: 2026-11-17 · Author: Audit-Hawk · Target: Arch Linux / Omarchy 4.5 (Hyprland, Wayland, tiling)
Repo: `/home/arch/dev/nodaysidle/nodaysidle-ocr-agent` (git, `main`, clean, origin `github.com/nodaysidle/nodaysidle-ocr-agent`, HEAD `022ff95`)

## Summary
1. Stack: Rust 2021 CLI (~360 LOC, tokio + clap) that chains `hyprpicker` (freeze) → `slurp` → `grim` → `tesseract` → formatter → `wl-copy`, with optional `--inject` into a Herdr/Tmux agent pane. No GUI window, no LLM/API calls.
2. Health: builds and 4/4 tests pass out of tree; 2 unused deps, `cargo fmt --check` fails. Main flow works, but every failure is invisible when launched from the keybind, and the "code reconstruction" misfires on ordinary prose.
3. Top risks: F01 silent failures from the keybind (no text / missing language / grim error → nothing happens), F02 `--inject` can paste multi-line OCR text into the wrong pane (loose substring match, all tmux sessions, no bracketed paste), F03 freeze overlay torn down before `grim` and orphaned if `slurp` fails to spawn.

## Stack Detected
| Part | Detail | Evidence |
|---|---|---|
| Language | Rust 2021, single crate: lib (`capture`, `ocr`, `formatter`, `dispatch`) + bin `nodaysidle-ocr-agent` | `Cargo.toml:1-7`, `src/lib.rs:1-4` |
| Deps | clap 4.5 (derive), tokio 1 `full`, serde (unused), serde_json, regex (unused), colored 2.1; dev: tempfile (unused) | `Cargo.toml:9-18` |
| Toolchain | cargo/rustc 1.100.0 | `versions.txt` |
| External tools | hyprpicker 0.4.7, slurp 1.5.0, grim 1.5.0, tesseract 5.5.3 (data: `eng`, `osd` only), wl-clipboard 2.3.0, optional herdr 0.9.5 / tmux 3.8, `omarchy-notification-send` → `notify-send` fallback | `pacman -Q`, `tesseract --list-langs` |
| Capture path | wlr-screencopy via `grim -g <slurp geometry> -` to stdout; no portal, no temp files (CLAUDE.md:15 rule holds: no `std::fs` writes anywhere in `src/`) | `src/capture.rs:10-51` |
| OCR | `tesseract stdin stdout --oem 1 --psm 6 -l <lang> --dpi 300 -c preserve_interword_spaces=1`, stderr discarded | `src/ocr.rs:10-24` |
| Output | `wl-copy` always; `--inject` → `herdr pane send-text` or `tmux set-buffer` + `paste-buffer`; `--stdout` | `src/dispatch.rs:18-38,99-130`, `src/main.rs:65-89` |
| Install | `~/.local/bin/nodaysidle-ocr-agent` (2.0 MB, 2026-11-08) + symlink `way-ocr-agent` | `ls -la ~/.local/bin` |
| Keybind | `SUPER + SHIFT + O` in `~/.config/hypr/bindings.lua:134-135`; `hyprctl binds -j` shows only this bind on that key, so Omarchy's default Obsidian bind (`/usr/share/omarchy/default/hypr/bindings/applications.lua:22`) is silently replaced | `hyprctl binds -j` |
| Packaging | `packaging/PKGBUILD` (source, `sha256sums=('SKIP')`), `packaging/PKGBUILD.bin` (sha256 matches `dist/*.tar.gz`) | `packaging/*`, `dist/*.sha256` |
| Omarchy overlap | Omarchy 4.5 ships `omarchy-capture-text` (same pipeline, `OMARCHY_OCR_LANGS`, bound to `SUPER + CTRL + PRINT`). This repo's added value is the formatter and `--inject`. | `/usr/bin/omarchy-capture-text:1-26`, `utilities.lua:46` |
| Omarchy/Hyprland checks | No window, so CSD/SSD, window rules, tiling sizing, socket2/polling, live theme reload and Accessible roles are N/A. What does apply: slurp overlay colors (not themed, I06), notifications (F01/F10), keybind (F13), HiDPI capture scale (F08; Alan's DP-1 is 2560x1440 @ scale 1, fractional untested), `.desktop` discoverability (I10). Theme key set checked across all 23 bundled themes (5 light: catppuccin-latte, flexoki-light, lupine, rose-pine, white): `accent`, `selection`, `background`, `foreground` exist in all 23, so I06 needs no fallbacks. | `/usr/share/omarchy/themes/*/colors.toml` |

### AI / API integration map
| Item | Value | Where |
|---|---|---|
| LLM / HTTP APIs | **None.** No HTTP client, no `env::var`, no endpoints, no model ids, no keys. `rg -n -i 'anthropic\|openai\|api[_-]?key\|https?://\|env::var\|reqwest\|TYPESAFE' src Cargo.toml` → only `"claude"` as a pane name to match. | `src/dispatch.rs:15` |
| Anthropic | No Anthropic API code. `"claude"` in `PREFERRED_AGENTS` only matches a Claude Code terminal pane for `--inject`; it calls nothing. Alan decides whether to drop it from the list. | `src/dispatch.rs:15` |
| TypeSafe (Jev) | Not used, not needed. | — |
| Failure UX for external tools | grim/tesseract/wl-copy failures → `eprintln!` + exit 1, invisible from the keybind (F01); tesseract stderr is discarded (F05); no timeouts (F22). | `src/main.rs:38-53`, `src/ocr.rs:22` |

## Analyzers Run / Missing
Output: `/tmp/audit-nodaysidle-ocr-agent-2026-11-17/`
| Analyzer | Result | File |
|---|---|---|
| `cargo build --offline --locked` (`CARGO_TARGET_DIR=/tmp/...`) | OK | `build.txt` |
| `cargo test --offline --locked` | 4/4 pass (formatter only) | `test.txt` |
| `cargo clippy --all-targets` (default) | 1 warning (`collapsible_if` formatter.rs:51) + 2 manifest warnings: unused deps `serde`, `regex` | `clippy-default.txt` |
| `cargo clippy --all-targets -- -W clippy::pedantic` | 32 warning lines, mostly `uninlined_format_args` (15) and missing `# Errors` docs (4) | `clippy.txt` |
| `cargo fmt --check` | FAIL, 10 hunks (capture.rs, dispatch.rs, formatter.rs) | `fmt.txt` |
| `namcap PKGBUILD PKGBUILD.bin` | 1 W: `x86_64` → `$CARCH` in PKGBUILD.bin | `namcap.txt` |
| Formatter probe (scratch crate in /tmp linking the lib) | 7/9 prose/code samples misclassified (F06) | `formatter-probe.txt` |
| tesseract probe on a rendered 14 pt code image (same args) | 0.21–0.26 s per run; leading indentation dropped; `fn`→`©0`, `println!`→`printint`; 2x upscale fixes `fn` (F07, F08, F09) | `timing.txt`, `sample-ocr.txt`, `sample2x-ocr.txt` |
| `tesseract -l slv` | `Failed loading language 'slv'` (no `tesseract-data-slv`) | `tess-slv.txt` |
| `hyprctl binds -j`, `herdr pane list` key set (read-only) | 1 bind on SUPER+SHIFT+O; herdr 0.9.5 panes have no `agent` key (F02) | n/a |
| **Missing** | `cargo-audit` / `cargo-deny` (not installed), `cargo-udeps` (cargo's own unused-dep lint covered it), shellcheck (no shell scripts). The app was **not run** (it freezes the screen and needs interactive selection), so capture/inject findings are from code + tool help. Orca/AT-SPI N/A (no window). | |

## Prior Handoff Status
None. No `HANDOFF.md` existed before this audit, so nothing was renamed.

## Findings
| ID | Sev | Area | file:line | Problem | Fix | Acceptance check |
|---|---|---|---|---|---|---|
| F01 | P1 | ux | `src/main.rs:38-53,66-68`, `src/ocr.rs:35-42`, `src/dispatch.rs:133-157` | Launched from the Hyprland keybind there is no terminal, so every failure (no text found, missing language data, grim/tesseract crash, wl-copy failure) only goes to stderr and exits. The user presses the key, drags, and nothing happens. `notify()` runs only on success. | Add `notify_error(msg)` using `omarchy-notification-send` (fallback `notify-send -u critical`) and call it on every error path except user cancel. "No text found in the selected region" gets its own message and exit code 2. Keep stderr output. | `rg -n 'notify_error' src/main.rs` shows a call in each `Err` branch (≥3 hits); new unit test maps each error variant to a message; `cargo test` passes |
| F02 | P1 | code/security | `src/dispatch.rs:15,41-96,84,99-130` | `--inject` picks the target by substring: `"pi"` matches any tmux command or herdr title containing "pi" (`pipx`, `spinner`, `~/pictures`). Tmux scans `list-panes -a` (every session) and takes the first hit, not the active pane. Herdr 0.9.5 `pane list` has no `agent` key (keys: `agent_status, cwd, focused, …, terminal_title`), so only the loose title match works. Multi-line OCR text pasted into a shell pane runs each line (no bracketed paste). | Exact, case-insensitive match on the command basename (tmux `#{pane_current_command}`) or the herdr title's first token. Prefer the focused pane (`herdr pane current`, tmux `#{pane_active}` in the attached session). Refuse targets whose command is `bash`/`zsh`/`fish`/`sh`. Use `tmux paste-buffer -p` (bracketed). Put the matcher in a pure fn and test it. | Unit tests: `matches_agent("pipx","pi")==false`, `matches_agent("pi","pi")==true`, shell commands rejected; `rg -n 'paste-buffer.*"-p"' src/dispatch.rs` hits; `cargo test` passes |
| F03 | P1 | code/ux | `src/capture.rs:10-29,41-45` | The `hyprpicker -r -z` freeze is killed (`:27-29`) **before** `grim` runs (`:41`), so grim captures the live screen, not the frozen frame the user selected (video, scrolling, cursor blink). Omarchy's own `omarchy-capture-text:7-12` keeps it alive until after grim for this reason. If `slurp` fails to spawn, `?` at `:24` returns early and the hyprpicker child is orphaned (tokio doesn't kill on drop), so the screen stays frozen. Spawn failure of hyprpicker is swallowed by `.ok()` (`:15`); fixed 50 ms sleep (`:18`, Omarchy uses 100 ms). | Wrap hyprpicker in a guard (`kill_on_drop(true)` + explicit `kill().await` after grim). Kill only after `grim` returns, on every path. Log once if hyprpicker is missing. Sleep 100 ms. | `rg -n 'kill_on_drop\(true\)' src/capture.rs` hits; in `capture_region` the `grim` call line number < the hyprpicker kill line (`rg -n 'Command::new\("grim"\)\|\.kill\(' src/capture.rs`) |
| F04 | P2 | code/ocr | `src/main.rs:14-16`, `packaging/PKGBUILD:11-15`, `packaging/PKGBUILD.bin:10-14` | Language is hardcoded to `eng`. Omarchy's convention `OMARCHY_OCR_LANGS` (used by `omarchy-capture-text:21`) is ignored. Only `eng`+`osd` traineddata are installed, so Slovenian text gets English-only recognition (č/š/ž lost), and `-l slv` fails. | Default `--lang` from `OMARCHY_OCR_LANGS`, else `eng` (clap `env = "OMARCHY_OCR_LANGS"`). Before running, check requested langs against `tesseract --list-langs` and fail with `install tesseract-data-<lang>`. Add `tesseract-data-slv` to optdepends. Alan decides whether to install it and whether the default becomes `eng+slv`. | `rg -n 'OMARCHY_OCR_LANGS' src/main.rs` hits; `rg -n 'tesseract-data-slv' packaging/` hits; unit test for the missing-lang message |
| F05 | P2 | code | `src/ocr.rs:22,35-37`, `src/capture.rs:47-48` | tesseract stderr goes to `/dev/null` and grim's stderr is captured but dropped, so errors say only "Tesseract failed to extract text" / "Failed to capture screen region with grim". The real cause (`Failed loading language 'slv'`, screencopy denied, bad geometry) is lost. | Pipe stderr, include its last non-empty line in the error (and in the F01 notification). | `rg -n 'Stdio::null\(\)' src/ocr.rs; echo exit=$?` → `exit=1`; `rg -n 'stderr' src/capture.rs src/ocr.rs` shows stderr included in `Err` |
| F06 | P2 | code | `src/formatter.rs:37-104` | Detection is bare substring checks. Probe (`formatter-probe.txt`): "Download … from the release page." → python; "# Release notes…" → bash; "…which match you prefer." → rust; "Use git to clone the repo." → bash; "Undefined reference to the old spec…" → Diagnostic; Python `match` → rust; JS `import {…} from` → python. Prose is wrapped in wrong fences. | Score per language with line-anchored patterns (`^\s*def \w+\(.*\):$`, `^from \S+ import `, `^\s*(pub )?fn \w+`, `^\s*(const\|let) \w+ =`, `^\$ `), need ≥2 signals and a non-prose ratio (symbols per word) before fencing. Anchor diagnostic markers. Check JS/TS `import {` before Python. | Add the 9 probe strings from `formatter-probe.txt` as tests with expected kinds (prose stays `Prose`, Python `match` → python, JS import → typescript); `cargo test` passes |
| F07 | P2 | code/docs | `README.md:64`, `docs/RELEASE_NOTES.md:26`, `src/formatter.rs:28-31` | "Preserves indentation depth" is false: `--psm 6` text output drops leading whitespace (`sample-ocr.txt`: `    let mut total` → `Tet mut total`). Code arrives flush-left. | Use `tesseract … tsv` and rebuild indentation from each line's `left` px divided by median char width, or remove the claim. Alan picks; default to rebuilding (it's the tool's main value). | Fixture test with a checked-in TSV sample producing 4-space indented output; or `rg -n -i 'indentation depth' README.md docs/; echo exit=$?` → `exit=1` |
| F08 | P2 | code/ocr | `src/capture.rs:41-43`, `src/ocr.rs:17` | Screen text (~14 px glyphs at scale 1, Alan's DP-1) goes to tesseract at native size with a fake `--dpi 300`. Probe: native gives `©0 main()`, 2x upscale gives `fn main()`. | Capture with `grim -s 2` (scale factor, see `grim -h`) when the output scale is < 2. Keep `--dpi 300` only with upscaled input. Optional `-t ppm` to skip PNG encode/decode. | `rg -n '"-s"' src/capture.rs` hits; re-run the probe from Verification on `sample2x.png` and get `fn main() {` on line 1 |
| F09 | P2 | docs/perf | `README.md:20`, `docs/RELEASE_NOTES.md:16` | "OCR in under 40 milliseconds" / "sub-40ms": tesseract alone took 0.21–0.26 s on a 900x200 region here (`timing.txt`). | Replace with a measured figure or remove. | `rg -n -i '40 ?m(s\|illi)' README.md docs/; echo exit=$?` → `exit=1` |
| F10 | P2 | ux | `src/main.rs:71-84`, `src/dispatch.rs:122,133-144` | The notification says "injected to <agent>" even when injection failed (target is set before inject and never reset), and `tmux paste-buffer`'s status is ignored. With `--inject` and no pane found it says "Copied to clipboard" with no hint that nothing was injected. | Return the real outcome from `inject_to_agent` and notify from it: `injected`, `inject failed: <reason>`, or `no agent pane found, clipboard only`. Check `paste-buffer` status. | Unit test for the message builder covering the 3 outcomes; `rg -n 'let _ = Command::new\("tmux"\)' src/dispatch.rs; echo exit=$?` → `exit=1` |
| F11 | P2 | code/security | `src/dispatch.rs:103,116` | OCR text goes to `herdr pane send-text <id> <TEXT>` and `tmux set-buffer <TEXT>` as an argv element. Text starting with `-` is parsed as an option, and anything over 128 KiB (MAX_ARG_STRLEN) fails. | tmux: `load-buffer -` with text on stdin, then `paste-buffer -p -t`. herdr: put `--` before the text. | `rg -n 'load-buffer' src/dispatch.rs` hits; `rg -n '"set-buffer"' src/dispatch.rs; echo exit=$?` → `exit=1`; `rg -n '"--"' src/dispatch.rs` hits |
| F12 | P2 | ux | `src/main.rs:31-36` | No single-instance guard. A double press of the key starts two hyprpicker freezes and two slurp selectors on top of each other. | `flock` (non-blocking) on `$XDG_RUNTIME_DIR/nodaysidle-ocr-agent.lock`; a second instance exits 0 silently. | `rg -n 'XDG_RUNTIME_DIR' src/` hits; unit test that the second lock attempt fails |
| F13 | P3 | ux/docs | `README.md:18,67,101-108`, `~/.config/hypr/bindings.lua:134-135` (Alan's) | README claims "Zero Key Conflicts", but `SUPER + SHIFT + O` is Omarchy's default Obsidian bind (`applications.lua:22`, Obsidian is installed). Alan's bindings.lua doesn't `hl.unbind` it first (its own header at `:18` says to). At runtime only the OCR bind remains, so Obsidian's shortcut is lost silently. README also doesn't mention Omarchy's built-in `omarchy-capture-text` (SUPER + CTRL + PRINT). | README: drop "Zero Key Conflicts", show `hl.unbind("SUPER + SHIFT + O")` before the bind, and add a short "vs omarchy-capture-text" section. Agent does **not** edit Alan's hypr config; Alan picks the key. | `rg -n 'Zero Key Conflicts' README.md; echo exit=$?` → `exit=1`; `rg -n 'hl.unbind' README.md` hits; `rg -n 'omarchy-capture-text' README.md` hits |
| F14 | P3 | build | `Cargo.toml:11-12,14,18` | Unused deps `serde`, `regex` (cargo warns), unused dev-dep `tempfile`; tokio `full` pulls in net/fs/signal for a process-only CLI. | Remove the three; tokio features `rt, macros, process, io-util, time`. Needs `cargo update`-free edit only (no new deps). | `cargo clippy --offline --all-targets 2>&1 \| rg -c 'unused dependency'; echo exit=$?` → `exit=1`; build passes |
| F15 | P3 | code | `src/main.rs:9-12` | Version, author and about are hardcoded (`"0.1.0"`) and will drift from `Cargo.toml`. | `#[command(version, author, about)]` (reads Cargo metadata). | `rg -n '"0\.1\.0"' src/; echo exit=$?` → `exit=1` |
| F16 | P3 | code | `src/main.rs:39`, `src/capture.rs:32`, all `Result<_, String>` | Errors are strings; cancel detection compares `err == "Selection cancelled"`. A wording change breaks the exit-0-on-cancel path. | `enum OcrError { Cancelled, MissingTool(&'static str), NoText, Tool { name, stderr } … }` with `Display`. No new crates. | `rg -n '"Selection cancelled"' src/main.rs; echo exit=$?` → `exit=1`; `cargo test` passes |
| F17 | P3 | code | `src/main.rs:63` | Word count is taken from the fenced text, so it counts the ```` ```lang ```` and ```` ``` ```` lines (probe: 8-word sentence → "10 words"). | Count `raw_text` words. | `rg -n 'raw_text.split_whitespace' src/main.rs` hits |
| F18 | P3 | build | `src/capture.rs:32`, `src/dispatch.rs:12-133`, `src/formatter.rs:51,93` | `cargo fmt --check` fails (10 hunks); clippy `collapsible_if` at `formatter.rs:51`. | `cargo fmt`; collapse the `if`. | `cargo fmt --check; echo exit=$?` → `exit=0`; `cargo clippy --offline --all-targets -- -D warnings` passes |
| F19 | P3 | docs | `README.md:89-90`, `CLAUDE.md:10` | Build-from-source copies `target/release/way-ocr-agent`, which cargo never builds (one bin; the symlink exists only in the release tarball and PKGBUILD, so `docs/RELEASE_NOTES.md:42` is correct). | Replace with `ln -sf nodaysidle-ocr-agent ~/.local/bin/way-ocr-agent`. | `rg -n 'release/way-ocr-agent' README.md CLAUDE.md; echo exit=$?` → `exit=1` |
| F20 | P3 | build | `packaging/PKGBUILD:9-19`, `packaging/PKGBUILD.bin:9,17` | Source PKGBUILD has `sha256sums=('SKIP')` for a release tarball. `tesseract-data-eng` is only optdepend though `eng` is the default. `hyprpicker` is a hard depend though the code tolerates its absence. namcap: `x86_64` → `$CARCH` in PKGBUILD.bin source. | Real checksum after the v0.1.0 tag is published (Alan); move `tesseract-data-eng` to depends; `hyprpicker` to optdepends; use `$CARCH`. | `rg -n "SKIP" packaging/PKGBUILD; echo exit=$?` → `exit=1`; `cd packaging && namcap PKGBUILD PKGBUILD.bin` → no output |
| F21 | P3 | code | `src/ocr.rs:32`, `src/dispatch.rs:43,72-76,102-106` | No timeouts on tesseract, herdr, tmux, wl-copy. A hung herdr/tmux server blocks the binary forever (and with F12 blocks later presses). | `tokio::time::timeout` (tesseract 20 s, herdr/tmux/wl-copy 3 s), killing the child on timeout. | `rg -c 'timeout' src/ocr.rs src/dispatch.rs` ≥ 1 each |
| F22 | P3 | code/tests | `tests/formatter_tests.rs:1-34` | Only the formatter is tested; capture/ocr/dispatch call `Command::new` directly with no seam. | Extract pure functions (arg builders, pane matcher F02, message builders F01/F10, TSV indenter F07) and test those. No mocking crates. | `cargo test 2>&1 \| rg 'test result'` shows ≥ 15 tests passing |

## Improvements / UI-UX Ideas
Ideas only. Not tasks unless Alan asks.
- I01 (idea) After capture, a Walker menu: Copy / Inject into <focused agent pane> / Open in editor, instead of choosing via flags. `src/main.rs:65-81`
- I02 (idea) Notification action buttons ("Inject", "Edit") via `notify-send -A`. `src/dispatch.rs:146-156`
- I03 (idea) `--lang` quick toggle: second keybind for `eng+slv`, or reuse `OMARCHY_OCR_LANGS` (F04). `src/main.rs:14-16`
- I04 (idea) Preview/edit step: floating Ghostty with `$EDITOR` on the OCR text before it hits the clipboard (`--review`), with a Hyprland float+center rule snippet for Alan. `src/main.rs:56-61`
- I05 (idea) Use `omarchy-i18n` strings for notifications, like `omarchy-capture-text:26`. `src/dispatch.rs:134-144`
- I06 (idea) Theme the slurp overlay from `~/.config/omarchy/current/theme/colors.toml` (`slurp -b/-c/-s`, keys `accent`/`selection`/`background` exist in all 23 bundled themes, dark and light). `src/capture.rs:21`
- I07 (idea) `--window`: OCR the focused window from `hyprctl activewindow -j` geometry, no drag. `src/capture.rs:8`
- I08 (idea) `--last`: re-OCR the last geometry (stored in `$XDG_RUNTIME_DIR`). `src/capture.rs:35`
- I09 (idea) Mark clipboard entries as sensitive or skip history for a `--private` capture, so OCR'd secrets don't land in clipboard history. `src/dispatch.rs:18-38`
- I10 (idea) Ship a `.desktop` file so it appears in Walker without the keybind. `packaging/PKGBUILD:40-46`

## Agent Task List
Do in order. One finding per change. Paste each proof command's output in your report. Mark done/blocked per ID.
1. F18, F14: `cargo fmt`, collapse the `if`, drop unused deps. Proof: F18 + F14 acceptance; out-of-tree build passes.
2. F16: typed errors (unblocks F01/F05). Proof: F16 acceptance.
3. F01, F05: error notifications with real tool stderr. Proof: F01 + F05 acceptance.
4. F03: hyprpicker guard and kill-after-grim. Proof: F03 acceptance. Live check needs Alan (press the key).
5. F02, F11, F10: safe inject (matcher, focused pane, bracketed paste, stdin buffers, honest outcome). Proof: F02, F11, F10 acceptance.
6. F04: language from `OMARCHY_OCR_LANGS`, missing-lang message, optdepends. Proof: F04 acceptance.
7. F08: `grim -s 2`. Proof: F08 acceptance.
8. F06: formatter scoring. Proof: F06 tests pass.
9. F07: indentation from TSV (or claim removed if Alan says so). Proof: F07 acceptance.
10. F12, F21: lock + timeouts. Proof: F12 + F21 acceptance.
11. F17, F15: word count, Cargo metadata. Proof: acceptance commands.
12. F09, F13, F19: docs. Proof: acceptance `rg` commands.
13. F20: packaging (checksum needs the published tag; leave `SKIP` and mark blocked if not yet released). Proof: `namcap` clean.
14. F22: tests. Proof: ≥15 tests pass.

## Verification
Run from the repo root. Build out of tree only.
```
export CARGO_TARGET_DIR=/tmp/nds-ocr-build
cargo build --offline --locked
cargo test --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
cargo fmt --check
(cd packaging && namcap PKGBUILD PKGBUILD.bin)
# OCR probe (no capture, no app run): reuse the audit images
A=/tmp/audit-nodaysidle-ocr-agent-2026-11-17
tesseract $A/sample2x.png stdout --oem 1 --psm 6 -l eng --dpi 300 -c preserve_interword_spaces=1
# key hygiene / no AI vendors: must print nothing (exit=1)
rg -n -i 'anthropic|openai|api[_-]?key|sk-[a-z0-9]' src Cargo.toml; echo exit=$?
```

## Out of Scope
- Alan's `~/.config/hypr/bindings.lua` (keybind, `hl.unbind`), `~/.local/bin` install, and installing `tesseract-data-slv`: give him the commands, don't change them.
- `target/` and `dist/` in the repo (gitignored build outputs); publishing a release or tag; pushing.
- Running the app (it freezes the screen and needs interactive selection) and `--inject` against Alan's live herdr/tmux panes.
- Adding any AI vendor. The tool calls no LLM. The only allowed extra integration is TypeSafe (Jev) via `TYPESAFE_API_KEY`, and none is needed. No Anthropic API code exists to remove; `"claude"` in `src/dispatch.rs:15` is a pane name.
- Outside the repo, noted only: `~/.config/hypr/bindings.lua:125-126` binds "Writing Tools (… Anthropic Claude Haiku)" to `SUPER + E`. Not part of this repo.
- Repo agent rules that conflict with this handoff (ignore them): `CLAUDE.md:7-8` (`cargo build` / `cargo build --release` in-tree; use `CARGO_TARGET_DIR=/tmp/...`), `CLAUDE.md:10` (install by copying into `~/.local/bin`; Alan installs), `CLAUDE.md:14` ("clean warnings": do it per finding F14/F18, not as silent extra edits).
