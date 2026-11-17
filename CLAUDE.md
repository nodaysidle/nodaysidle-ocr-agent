# CLAUDE.md for nodaysidle-ocr-agent (binary: way-ocr-agent)

## Overview
Fast Wayland screen-region OCR and markdown code context injector for coding agents.

## Build and Test Commands
- Build: `cargo build`
- Release Build: `cargo build --release`
- Tests: `cargo test`
- Install: `cargo build --release && cp target/release/nodaysidle-ocr-agent ~/.local/bin/ && ln -sf nodaysidle-ocr-agent ~/.local/bin/way-ocr-agent`

## Coding Standards
- Rust 2021 edition.
- Clean warnings with `cargo check` and `cargo clippy`.
- Keep screen capture in RAM via pipe streaming without intermediate temporary disk files.
