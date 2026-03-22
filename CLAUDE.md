# Kora — Multi-Agent Development Orchestration CLI

## Build & Test

- `cargo build` — compile
- `cargo test` — run all tests (155 tests across 21 test files)
- `cargo clippy -- -D warnings` — lint (must be clean)
- `cargo fmt --check` — format check
- `cargo run` — run interactive session
- `cargo run -- configure` — run setup wizard
- `cargo run -- run "request"` — one-shot mode

## Architecture

Rust binary. Ten AI agent roles orchestrated through a pipeline:
Researcher → Plan Reviewer + Plan Security Auditor (parallel) → Judge → Planner → Test Architect → Implementors (parallel fleet) → Code Reviewer + Code Security Auditor (parallel, per task) → Validator

### Module layout
- `src/cli/` — clap CLI definition, configure wizard, resume/history/clean commands, meta commands
- `src/config/` — YAML config schema, merged loading (project `.kora/config.yml` + home `~/.kora/config.yml`, home takes precedence)
- `src/state/` — Stage enum with transition validation, RunState persistence, run directory management
- `src/provider/` — Provider trait, Claude + Codex CLI adapters, provider detection
- `src/agent/` — prompt loading via `include_str!()` from `prompts/`, structured output parser (verdict/review/validation markers)
- `src/pipeline/` — orchestrator, review loop, researcher session, planner, test architect, implementor fleet, validation loop, merge flow
- `src/terminal/` — renderer, arrow-key selector, text input, verbosity modes, implementation dashboard
- `src/git/` — worktree create/remove/merge operations
- `prompts/` — agent prompt markdown files compiled into binary at build time

## Conventions

- Provider-agnostic: never call AI APIs directly, always spawn CLI tools (`claude`, `codex`) as subprocesses
- Agents are stateless: communicate through files in run directory, orchestrator mediates
- CLI agents always run in yolo/autonomous mode (no permission prompts from underlying tools)
- Runs stored in `~/.kora/runs/`, never in project directory
- Config merging: project config for team defaults, home config for personal overrides
- Structured output: agents include `<!-- TAG -->...<!-- /TAG -->` markers parsed by output_parser.rs
- All state persisted to disk after every stage transition for resumability
- Tests use `tempfile::TempDir` for isolation — no test touches real `~/.kora/`
- `anyhow::Result` for error propagation, `thiserror` for custom error types
- Async runtime: tokio (full features) for parallel agent execution

## Testing

- Integration tests in `tests/` (one file per module)
- `assert_cmd` + `predicates` for CLI smoke tests
- Prompt assembly tests verify base prompt inclusion, context injection, custom instructions
- Output parser tests cover all marker formats (verdict, review, security, validation, task breakdown)
- State tests verify stage transitions, persistence roundtrips, checkpoint mapping

## Release Process

- PRs merge to `main` → release-drafter auto-updates a draft GitHub Release
- To release: edit draft on GitHub → set version tag (e.g. `v0.1.0`) → click Publish
- Publish triggers cross-compilation for macOS (arm64/x86) + Linux (arm64/x86)
- Binaries uploaded to GitHub Release automatically
- npm + crates.io publish gated by repo variables (`PUBLISH_NPM`, `PUBLISH_CRATES`)
