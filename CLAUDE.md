# Kora — Multi-Agent Development Orchestration CLI

## Build & Test

- `cargo build` — compile
- `cargo test` — run all tests
- `cargo clippy -- -D warnings` — lint (must be clean)
- `cargo fmt --check` — format check
- `cargo run` — run interactive session
- `cargo run -- configure` — run setup wizard
- `cargo run -- run "request"` — one-shot mode

## Architecture

Rust binary. Specialized AI agent roles orchestrated through an adaptive pipeline:
Researcher → Plan Reviewer + Plan Security Auditor (parallel) → Judge → Planner → Test Architect → Implementors (parallel fleet) → Code Reviewer + Code Security Auditor (parallel, per task) → Validator

### Pipeline profiles
The pipeline adapts to task complexity via `PipelineProfile` (Trivial/Simple/Standard/SecurityCritical). The Researcher auto-classifies each request using a `<!-- CLASSIFICATION -->` marker. Profiles control which stages execute by adjusting agent `enabled` flags via `apply_profile_to_config()`. CLI override: `--profile <name>`.

### Pipeline presets
Smart provider routing via `PipelinePreset` (Quality/Balanced/Speed/Custom). Presets assign the best available provider to each agent role based on detected CLI tools. All 7 provider combinations (claude, codex, gemini, and every combo) have tailored preset tables in `src/config/presets.rs`. Presets set provider only — model selection is left to each CLI's default. Custom mode allows per-agent provider:model override.

### Module layout
- `src/cli/` — clap CLI definition, configure wizard, resume/history/clean commands, meta commands
- `src/config/` — YAML config schema, merged loading, pipeline presets (provider routing)
- `src/state/` — Stage enum with transition validation, PipelineProfile (adaptive complexity), RunState persistence, run directory management
- `src/provider/` — Provider trait, Claude + Codex + Gemini CLI adapters, provider detection, model catalogs
- `src/agent/` — prompt loading via `include_str!()` from `prompts/`, structured output parser (verdict/review/validation markers)
- `src/pipeline/` — orchestrator, review loop, researcher session, planner, test architect, implementor fleet, validation loop, merge flow
- `src/terminal/` — renderer, selector (settings menu, preset panel, toggle list, multi-select, confirm dialog), input with command autocomplete, implementation dashboard
- `src/git/` — worktree create/remove/merge operations
- `prompts/` — agent prompt markdown files compiled into binary at build time

## Conventions

- Provider-agnostic: never call AI APIs directly, always spawn CLI tools (`claude`, `codex`, `gemini`) as subprocesses
- Agents are stateless: communicate through files in run directory, orchestrator mediates
- CLI agents always run in yolo/autonomous mode (no permission prompts from underlying tools)
- Runs stored in `~/.kora/runs/`, never in project directory
- Config merging: project config for team defaults, home config for personal overrides
- Structured output: agents include `<!-- TAG -->...<!-- /TAG -->` markers parsed by output_parser.rs (verdict, review, security, validation, classification)
- Interactive TUI runs in alternate screen with raw mode input, command autocomplete, and kora purple (#6C5CE7) brand color
- All state persisted to disk after every stage transition for resumability
- Tests use `tempfile::TempDir` for isolation — no test touches real `~/.kora/`; CLI tests set `HOME` to temp dir
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
