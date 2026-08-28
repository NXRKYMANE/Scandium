# Contributing Guide

Thank you for your interest in and contributions to Scandium!

## Project Structure Highlights

- **Single implementation**: a single Rust (edition 2024) implementation, artifact is `scandium_svc.exe` (entry `Project/main.rs`, core logic `Project/service_core.rs`).
- **Installer**: the Inno Setup 7 script lives at `Project/installer.iss`, built uniformly by `BUILD.ps1` (compile → publish → package).
- **Single source of version**: the `version` in `Project/Cargo.toml`, automatically synced by `BUILD.ps1` into `installer.iss`.

## Development Workflow

1. Fork this repository and create a feature branch
2. Modify the code
3. Verify locally: run `.\BUILD.ps1` (compile + package, 0 warnings 0 errors)
4. Commit and open a Pull Request

## Code Standards

- Comments span no more than two lines; fold long single-line comments into two lines
- After every edit, check: remove redundant / dead code, merge mergeable code, clean up unused `use` imports (Rust)
- When modifying the installer, keep [CustomMessages] bilingual in sync (english / chinesesimp)

## Commit Messages

Describe the change in clear English or Chinese, e.g. "Fix the installer log pane not scrolling to the bottom".
