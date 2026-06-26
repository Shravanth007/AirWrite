# AirWrite — Working Rules

## Git / GitHub workflow (non-negotiable)
- **Never commit to `main`. Never merge directly into `main`.** No exceptions.
- **Always work on a separate branch**, even for one-line changes (`fix/...`, `feat/...`, `chore/...`).
- **Every change ships as a GitHub Pull Request.** Push the branch, open a PR with `gh`, let it be reviewed/merged on GitHub.
- One logical change per branch/PR.

## How work gets done
- **Opus** does the thinking: planning, design decisions, and review. Use extended thinking for non-trivial reasoning.
- **Sonnet** does the coding: spawn coding agents on Sonnet to implement.
- After implementation, run **two verification passes**:
  1. **Post-coding verify** — confirm each change is correct and complete against its goal.
  2. **Complete review** — a fresh, full code review of the diff before the PR.
- Then run the app build (`npm run build`, plus `cargo build`/`cargo test` for the Rust side) to confirm everything actually works before pushing.

## Project shape (quick reference)
- Tauri 2 app: Rust backend in `src-tauri/src`, React/Vite frontend in `src`. Windows-only.
- Pipeline: global hotkey → record (cpal) → Groq Whisper → local cleanup → optional LLM cleanup → paste (Ctrl+V).
- Secrets: Groq API key lives in Windows Credential Manager (`settings.rs`), never in `config.json`.
- Source files must be **UTF-8 without BOM**, real Unicode glyphs only (no mojibake).
