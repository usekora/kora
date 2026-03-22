# Kora Phase 1: Foundation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Set up the Kora Rust project with CLI skeleton, configuration system, state machine, provider abstraction, and terminal rendering foundation — enough to run `kora` and `kora configure`.

**Architecture:** Rust binary using clap for CLI, serde for config/state serialization, crossterm + termimad for terminal UX, tokio for async. Providers are CLI subprocess adapters behind a trait. State is persisted to `.kora/runs/` as JSON/YAML files.

**Tech Stack:** Rust 1.80+, tokio, clap 4, serde, serde_json, serde_yaml, crossterm, termimad, indicatif, which, uuid, chrono, walkdir

**Spec:** `docs/specs/2026-03-22-kora-design.md`

---

## File Structure

```
kora/
├── Cargo.toml
├── install.sh                       ← shell installer script
├── prompts/                         ← agent prompt markdown files (compiled in via include_str!)
│   ├── researcher.md
│   ├── reviewer.md
│   ├── security_auditor.md
│   ├── judge.md
│   ├── planner.md
│   ├── test_architect.md
│   ├── implementor.md
│   └── validator.md
├── src/
│   ├── main.rs                      ← CLI entry point
│   ├── lib.rs                       ← public API re-exports
│   ├── cli/
│   │   ├── mod.rs                   ← CLI module
│   │   ├── app.rs                   ← clap App definition with subcommands
│   │   └── configure.rs             ← interactive configure wizard
│   ├── config/
│   │   ├── mod.rs                   ← config module, load/save
│   │   └── schema.rs                ← Config struct + serde
│   ├── state/
│   │   ├── mod.rs                   ← state module
│   │   ├── stage.rs                 ← Stage enum + transitions
│   │   ├── run.rs                   ← RunState struct + persistence
│   │   └── directory.rs             ← run directory creation + management
│   ├── provider/
│   │   ├── mod.rs                   ← provider module + registry
│   │   ├── traits.rs                ← Provider trait definition
│   │   ├── claude.rs                ← Claude CLI adapter
│   │   ├── codex.rs                 ← Codex CLI adapter
│   │   └── detection.rs             ← detect installed CLI tools
│   ├── terminal/
│   │   ├── mod.rs                   ← terminal module
│   │   ├── renderer.rs              ← stage headers, findings, verdicts
│   │   ├── selector.rs              ← arrow-key single/multi select
│   │   ├── input.rs                 ← freeform text input with > prompt
│   │   └── verbosity.rs             ← verbosity mode state + toggle
│   └── agent/
│       ├── mod.rs                   ← agent module
│       ├── prompts.rs               ← prompt loading (include_str!) + assembly
│       └── output_parser.rs         ← structured marker extraction
├── tests/
│   ├── config_test.rs               ← config load/save/defaults
│   ├── state_test.rs                ← stage transitions, run persistence
│   ├── provider_test.rs             ← provider detection, prompt building
│   ├── output_parser_test.rs        ← structured marker parsing
│   ├── verbosity_test.rs           ← verbosity cycle logic
│   └── selector_test.rs             ← selector logic (non-interactive)
└── .github/
    └── workflows/
        └── release.yml              ← cross-compile + GitHub Release + Homebrew
```

---

### Task 1: Initialize Rust Project

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Create: `.gitignore`
- Create: `rust-toolchain.toml`

- [ ] **Step 1: Initialize the project**

```bash
cd ~/dev/kora
git init
cargo init --name kora
```

- [ ] **Step 2: Set up rust-toolchain.toml**

Create `rust-toolchain.toml`:
```toml
[toolchain]
channel = "stable"
```

- [ ] **Step 3: Add dependencies to Cargo.toml**

Replace `Cargo.toml` with:
```toml
[package]
name = "kora"
version = "0.1.0"
edition = "2021"
description = "Multi-agent development orchestration CLI"
license = "MIT"
repository = "https://github.com/kora-ai/kora"

[dependencies]
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
crossterm = "0.27"
termimad = "0.29"
indicatif = "0.17"
which = "6"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
walkdir = "2"
anyhow = "1"
thiserror = "1"

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

- [ ] **Step 4: Set up .gitignore**

Create `.gitignore`:
```
/target
```

- [ ] **Step 5: Create minimal main.rs**

```rust
fn main() {
    println!("kora v0.1.0");
}
```

- [ ] **Step 6: Verify it compiles and runs**

Run: `cargo run`
Expected: prints `kora v0.1.0`

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "init: scaffold Kora Rust project with dependencies"
```

---

### Task 2: Configuration Schema

**Files:**
- Create: `src/config/mod.rs`
- Create: `src/config/schema.rs`
- Create: `tests/config_test.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write failing test for config defaults**

Create `tests/config_test.rs`:
```rust
use kora::config::Config;

#[test]
fn test_default_config_has_claude_provider() {
    let config = Config::default();
    assert_eq!(config.default_provider, "claude");
}

#[test]
fn test_default_config_has_checkpoints() {
    let config = Config::default();
    assert!(config.checkpoints.contains(&kora::state::Checkpoint::AfterResearcher));
    assert!(config.checkpoints.contains(&kora::state::Checkpoint::AfterPlanner));
}

#[test]
fn test_config_roundtrip_yaml() {
    let config = Config::default();
    let yaml = serde_yaml::to_string(&config).unwrap();
    let parsed: Config = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(config, parsed);
}
```

- [ ] **Step 2: Run tests to verify they don't compile yet**

Run: `cargo test --test config_test 2>&1 || true`
Expected: compilation error — module `config` not found. This is expected; we implement in the next steps.

- [ ] **Step 3: Implement config schema**

Create `src/config/schema.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::state::Checkpoint;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub version: u32,
    pub default_provider: String,
    pub providers: HashMap<String, ProviderConfig>,
    pub agents: AgentsConfig,
    pub checkpoints: Vec<Checkpoint>,
    pub review_loop: LoopConfig,
    pub validation_loop: LoopConfig,
    pub implementation: ImplementationConfig,
    pub output: OutputConfig,
    pub runs_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderConfig {
    pub cli: String,
    #[serde(default)]
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    pub custom_instructions: Option<PathBuf>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_provider() -> String {
    "default".to_string()
}

fn default_timeout() -> u64 {
    300
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentsConfig {
    pub researcher: AgentConfig,
    pub reviewer: AgentConfig,
    pub security_auditor: AgentConfig,
    pub judge: AgentConfig,
    pub planner: AgentConfig,
    pub test_architect: AgentConfig,
    pub implementor: AgentConfig,
    pub validator: AgentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoopConfig {
    pub max_iterations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImplementationConfig {
    pub branch_strategy: BranchStrategy,
    pub parallel_limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BranchStrategy {
    Separate,
    Single,
    PlannerDecides,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutputConfig {
    pub default_verbosity: Verbosity,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Verbosity {
    Focused,
    Detailed,
    Verbose,
}

impl Default for Config {
    fn default() -> Self {
        let default_agent = AgentConfig {
            provider: "default".to_string(),
            custom_instructions: None,
            timeout_seconds: 300,
        };

        let mut providers = HashMap::new();
        providers.insert(
            "claude".to_string(),
            ProviderConfig {
                cli: "claude".to_string(),
                flags: vec![],
            },
        );
        providers.insert(
            "codex".to_string(),
            ProviderConfig {
                cli: "codex".to_string(),
                flags: vec![],
            },
        );

        Self {
            version: 1,
            default_provider: "claude".to_string(),
            providers,
            agents: AgentsConfig {
                researcher: default_agent.clone(),
                reviewer: default_agent.clone(),
                security_auditor: default_agent.clone(),
                judge: default_agent.clone(),
                planner: default_agent.clone(),
                test_architect: default_agent.clone(),
                implementor: default_agent.clone(),
                validator: default_agent,
            },
            checkpoints: vec![Checkpoint::AfterResearcher, Checkpoint::AfterPlanner],
            review_loop: LoopConfig { max_iterations: 3 },
            validation_loop: LoopConfig { max_iterations: 2 },
            implementation: ImplementationConfig {
                branch_strategy: BranchStrategy::Separate,
                parallel_limit: 4,
            },
            output: OutputConfig {
                default_verbosity: Verbosity::Focused,
            },
            runs_dir: PathBuf::from(".kora/runs"),
        }
    }
}
```

Create `src/config/mod.rs`:
```rust
mod schema;

pub use schema::*;

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const CONFIG_DIR: &str = ".kora";
const CONFIG_FILE: &str = "config.yml";

pub fn config_path(project_root: &Path) -> PathBuf {
    project_root.join(CONFIG_DIR).join(CONFIG_FILE)
}

pub fn load(project_root: &Path) -> Result<Config> {
    let path = config_path(project_root);
    if !path.exists() {
        return Ok(Config::default());
    }
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read config at {}", path.display()))?;
    let config: Config = serde_yaml::from_str(&contents)
        .with_context(|| format!("failed to parse config at {}", path.display()))?;
    Ok(config)
}

pub fn save(project_root: &Path, config: &Config) -> Result<()> {
    let dir = project_root.join(CONFIG_DIR);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(CONFIG_FILE);
    let yaml = serde_yaml::to_string(config)?;
    std::fs::write(&path, yaml)?;

    let gitignore_path = dir.join(".gitignore");
    if !gitignore_path.exists() {
        std::fs::write(&gitignore_path, "runs/\n")?;
    }

    Ok(())
}
```

- [ ] **Step 4: Create stub state module for Checkpoint enum**

Create `src/state/mod.rs`:
```rust
mod stage;

pub use stage::*;
```

Create `src/state/stage.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Checkpoint {
    AfterResearcher,
    AfterReviewLoop,
    AfterPlanner,
    AfterImplementation,
}
```

- [ ] **Step 5: Wire up lib.rs**

Replace `src/lib.rs`:
```rust
pub mod config;
pub mod state;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test config_test`
Expected: 3 tests PASS

- [ ] **Step 7: Commit**

```bash
git add src/config/ src/state/ src/lib.rs tests/config_test.rs
git commit -m "feat: add configuration schema with defaults, load, and save"
```

---

### Task 3: State Machine

**Files:**
- Modify: `src/state/stage.rs`
- Create: `src/state/run.rs`
- Create: `src/state/directory.rs`
- Modify: `src/state/mod.rs`
- Create: `tests/state_test.rs`

- [ ] **Step 1: Write failing tests for stage transitions**

Create `tests/state_test.rs`:
```rust
use kora::state::{Stage, can_transition};

#[test]
fn test_researching_to_reviewing_is_valid() {
    assert!(can_transition(&Stage::Researching, &Stage::Reviewing));
}

#[test]
fn test_researching_to_security_auditing_is_valid() {
    assert!(can_transition(&Stage::Researching, &Stage::SecurityAuditing));
}

#[test]
fn test_researching_to_implementing_is_invalid() {
    assert!(!can_transition(&Stage::Researching, &Stage::Implementing));
}

#[test]
fn test_judging_to_researching_is_valid_for_revise() {
    assert!(can_transition(&Stage::Judging, &Stage::Researching));
}

#[test]
fn test_judging_to_planning_is_valid_for_approve() {
    assert!(can_transition(&Stage::Judging, &Stage::Planning));
}

#[test]
fn test_awaiting_approval_wraps_next_stage() {
    let stage = Stage::AwaitingApproval(Box::new(Stage::Reviewing));
    assert!(can_transition(&stage, &Stage::Reviewing));
}
```

- [ ] **Step 2: Run tests to verify they don't compile yet**

Run: `cargo test --test state_test 2>&1 || true`
Expected: compilation error — `Stage` type not found. This is expected.

- [ ] **Step 3: Implement Stage enum with transition validation**

Replace `src/state/stage.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Checkpoint {
    AfterResearcher,
    AfterReviewLoop,
    AfterPlanner,
    AfterImplementation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Researching,
    Reviewing,
    SecurityAuditing,
    Judging,
    Planning,
    TestArchitecting,
    Implementing,
    Validating,
    Fixing,
    AwaitingApproval(Box<Stage>),
    Complete,
    Failed(String),
}

impl Stage {
    pub fn label(&self) -> &str {
        match self {
            Stage::Researching => "researcher",
            Stage::Reviewing => "reviewer",
            Stage::SecurityAuditing => "security auditor",
            Stage::Judging => "judge",
            Stage::Planning => "planner",
            Stage::TestArchitecting => "test architect",
            Stage::Implementing => "implementing",
            Stage::Validating => "validator",
            Stage::Fixing => "fixing",
            Stage::AwaitingApproval(_) => "awaiting approval",
            Stage::Complete => "complete",
            Stage::Failed(_) => "failed",
        }
    }
}

pub fn can_transition(from: &Stage, to: &Stage) -> bool {
    match (from, to) {
        (Stage::Researching, Stage::Reviewing) => true,
        (Stage::Researching, Stage::SecurityAuditing) => true,
        (Stage::Researching, Stage::AwaitingApproval(_)) => true,
        (Stage::Reviewing, Stage::Judging) => true,
        (Stage::SecurityAuditing, Stage::Judging) => true,
        (Stage::Judging, Stage::Researching) => true,
        (Stage::Judging, Stage::Planning) => true,
        (Stage::Planning, Stage::TestArchitecting) => true,
        (Stage::TestArchitecting, Stage::Implementing) => true,
        (Stage::TestArchitecting, Stage::AwaitingApproval(_)) => true,
        (Stage::Implementing, Stage::Validating) => true,
        (Stage::Validating, Stage::Fixing) => true,
        (Stage::Validating, Stage::Complete) => true,
        (Stage::Fixing, Stage::Validating) => true,
        (Stage::AwaitingApproval(next), to) if next.as_ref() == to => true,
        _ => false,
    }
}
```

- [ ] **Step 4: Implement RunState and persistence**

Create `src/state/run.rs`:
```rust
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::Stage;
use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunState {
    pub id: String,
    pub request: String,
    pub status: Stage,
    pub current_iteration: u32,
    pub provider_overrides: HashMap<String, String>,
    pub timestamps: HashMap<String, DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub error: Option<String>,
}

impl RunState {
    pub fn new(request: &str) -> Self {
        let now = Utc::now();
        let mut timestamps = HashMap::new();
        timestamps.insert("created".to_string(), now);

        Self {
            id: Uuid::new_v4().to_string()[..8].to_string(),
            request: request.to_string(),
            status: Stage::Researching,
            current_iteration: 0,
            provider_overrides: HashMap::new(),
            timestamps,
            created_at: now,
            updated_at: now,
            error: None,
        }
    }

    pub fn save(&self, runs_dir: &Path) -> Result<()> {
        let run_dir = runs_dir.join(&self.id);
        std::fs::create_dir_all(&run_dir)?;
        let path = run_dir.join("state.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;

        let request_path = run_dir.join("request.md");
        if !request_path.exists() {
            std::fs::write(&request_path, &self.request)?;
        }

        Ok(())
    }

    pub fn load(runs_dir: &Path, run_id: &str) -> Result<Self> {
        let path = runs_dir.join(run_id).join("state.json");
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read run state at {}", path.display()))?;
        let state: Self = serde_json::from_str(&contents)?;
        Ok(state)
    }

    pub fn advance(&mut self, next: Stage) {
        self.status = next;
        self.updated_at = Utc::now();
        self.timestamps
            .insert(self.status.label().to_string(), self.updated_at);
    }
}
```

- [ ] **Step 5: Implement run directory management**

Create `src/state/directory.rs`:
```rust
use anyhow::Result;
use std::path::{Path, PathBuf};

use super::RunState;

pub struct RunDirectory {
    base: PathBuf,
}

impl RunDirectory {
    pub fn new(runs_dir: &Path, run_id: &str) -> Self {
        Self {
            base: runs_dir.join(run_id),
        }
    }

    pub fn create_structure(&self) -> Result<()> {
        let dirs = [
            "context",
            "reviews",
            "plan",
            "implementation",
            "validation",
        ];
        for dir in dirs {
            std::fs::create_dir_all(self.base.join(dir))?;
        }
        Ok(())
    }

    pub fn context_dir(&self) -> PathBuf {
        self.base.join("context")
    }

    pub fn reviews_dir(&self, iteration: u32) -> PathBuf {
        self.base.join("reviews").join(format!("iteration-{}", iteration))
    }

    pub fn plan_dir(&self) -> PathBuf {
        self.base.join("plan")
    }

    pub fn task_dir(&self, task_id: &str) -> PathBuf {
        self.base.join("implementation").join(format!("task-{}", task_id))
    }

    pub fn validation_dir(&self) -> PathBuf {
        self.base.join("validation")
    }

    pub fn list_interrupted(runs_dir: &Path) -> Result<Vec<RunState>> {
        let mut interrupted = Vec::new();
        if !runs_dir.exists() {
            return Ok(interrupted);
        }
        for entry in std::fs::read_dir(runs_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let run_id = entry.file_name().to_string_lossy().to_string();
                if let Ok(state) = RunState::load(runs_dir, &run_id) {
                    match state.status {
                        crate::state::Stage::Complete | crate::state::Stage::Failed(_) => {}
                        _ => interrupted.push(state),
                    }
                }
            }
        }
        interrupted.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(interrupted)
    }
}
```

- [ ] **Step 6: Update state module exports**

Replace `src/state/mod.rs`:
```rust
mod stage;
mod run;
mod directory;

pub use stage::*;
pub use run::RunState;
pub use directory::RunDirectory;
```

- [ ] **Step 7: Run tests**

Run: `cargo test --test state_test`
Expected: 5 tests PASS

- [ ] **Step 8: Add persistence roundtrip test**

Add to `tests/state_test.rs`:
```rust
use kora::state::RunState;
use tempfile::TempDir;

#[test]
fn test_run_state_save_and_load() {
    let tmp = TempDir::new().unwrap();
    let runs_dir = tmp.path();

    let state = RunState::new("add dark mode support");
    state.save(runs_dir).unwrap();

    let loaded = RunState::load(runs_dir, &state.id).unwrap();
    assert_eq!(loaded.id, state.id);
    assert_eq!(loaded.request, "add dark mode support");
}
```

- [ ] **Step 9: Run all tests**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 10: Commit**

```bash
git add src/state/ tests/state_test.rs
git commit -m "feat: add state machine with stage transitions, run persistence, and directory management"
```

---

### Task 4: Provider Abstraction

**Files:**
- Create: `src/provider/mod.rs`
- Create: `src/provider/traits.rs`
- Create: `src/provider/claude.rs`
- Create: `src/provider/codex.rs`
- Create: `src/provider/detection.rs`
- Create: `tests/provider_test.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write failing tests for provider detection**

Create `tests/provider_test.rs`:
```rust
use kora::provider::{detect_providers, ProviderKind};

#[test]
fn test_provider_kind_cli_name() {
    assert_eq!(ProviderKind::Claude.cli_name(), "claude");
    assert_eq!(ProviderKind::Codex.cli_name(), "codex");
}

#[test]
fn test_provider_kind_autonomous_flags() {
    let flags = ProviderKind::Claude.autonomous_flags();
    assert!(flags.contains(&"--dangerously-skip-permissions"));

    let flags = ProviderKind::Codex.autonomous_flags();
    assert!(flags.contains(&"--approval-mode"));
    assert!(flags.contains(&"full-auto"));
}

#[test]
fn test_provider_kind_has_non_interactive_flags() {
    let flags = ProviderKind::Claude.non_interactive_flags();
    assert!(flags.contains(&"--print"));

    let flags = ProviderKind::Codex.non_interactive_flags();
    assert!(flags.contains(&"--quiet"));
}
```

- [ ] **Step 2: Run tests to verify they don't compile yet**

Run: `cargo test --test provider_test 2>&1 || true`
Expected: compilation error — module `provider` not found. This is expected.

- [ ] **Step 3: Implement provider trait**

Create `src/provider/traits.rs`:
```rust
use anyhow::Result;
use std::path::Path;

pub struct AgentOutput {
    pub text: String,
    pub exit_code: i32,
    pub duration: std::time::Duration,
}

pub struct InteractiveSession {
    pub child: tokio::process::Child,
}

#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> ProviderKind;
    fn is_available(&self) -> Result<bool>;

    async fn run(
        &self,
        prompt: &str,
        working_dir: &Path,
        extra_flags: &[String],
    ) -> Result<AgentOutput>;

    async fn run_interactive(
        &self,
        prompt: &str,
        working_dir: &Path,
        extra_flags: &[String],
    ) -> Result<InteractiveSession>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Claude,
    Codex,
}

impl ProviderKind {
    pub fn cli_name(&self) -> &'static str {
        match self {
            ProviderKind::Claude => "claude",
            ProviderKind::Codex => "codex",
        }
    }

    pub fn autonomous_flags(&self) -> Vec<&'static str> {
        match self {
            ProviderKind::Claude => vec!["--dangerously-skip-permissions"],
            ProviderKind::Codex => vec!["--approval-mode", "full-auto"],
        }
    }

    pub fn non_interactive_flags(&self) -> Vec<&'static str> {
        match self {
            ProviderKind::Claude => vec!["--print"],
            ProviderKind::Codex => vec!["--quiet"],
        }
    }
}
```

Add `async-trait` to `Cargo.toml` dependencies:
```toml
async-trait = "0.1"
```

- [ ] **Step 4: Implement Claude provider**

Create `src/provider/claude.rs`:
```rust
use anyhow::Result;
use std::path::Path;
use std::process::Stdio;
use std::time::Instant;
use tokio::process::Command;

use super::traits::{AgentOutput, InteractiveSession, Provider, ProviderKind};

pub struct ClaudeProvider {
    cli_path: String,
}

impl ClaudeProvider {
    pub fn new(cli_path: &str) -> Self {
        Self {
            cli_path: cli_path.to_string(),
        }
    }

    fn base_command(&self, working_dir: &Path, extra_flags: &[String]) -> Command {
        let mut cmd = Command::new(&self.cli_path);
        cmd.current_dir(working_dir);
        for flag in ProviderKind::Claude.autonomous_flags() {
            cmd.arg(flag);
        }
        for flag in extra_flags {
            cmd.arg(flag);
        }
        cmd
    }
}

#[async_trait::async_trait]
impl Provider for ClaudeProvider {
    fn name(&self) -> &str {
        "claude"
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Claude
    }

    fn is_available(&self) -> Result<bool> {
        Ok(which::which(&self.cli_path).is_ok())
    }

    async fn run(
        &self,
        prompt: &str,
        working_dir: &Path,
        extra_flags: &[String],
    ) -> Result<AgentOutput> {
        let start = Instant::now();
        let mut cmd = self.base_command(working_dir, extra_flags);
        for flag in ProviderKind::Claude.non_interactive_flags() {
            cmd.arg(flag);
        }
        cmd.arg("-p").arg(prompt);
        let output = cmd.output().await?;
        let duration = start.elapsed();

        Ok(AgentOutput {
            text: String::from_utf8_lossy(&output.stdout).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            duration,
        })
    }

    async fn run_interactive(
        &self,
        prompt: &str,
        working_dir: &Path,
        extra_flags: &[String],
    ) -> Result<InteractiveSession> {
        let mut cmd = self.base_command(working_dir, extra_flags);
        cmd.arg("-p").arg(prompt);
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
        let child = cmd.spawn()?;
        Ok(InteractiveSession { child })
    }
}
```

- [ ] **Step 5: Implement Codex provider**

Create `src/provider/codex.rs`:
```rust
use anyhow::Result;
use std::path::Path;
use std::process::Stdio;
use std::time::Instant;
use tokio::process::Command;

use super::traits::{AgentOutput, InteractiveSession, Provider, ProviderKind};

pub struct CodexProvider {
    cli_path: String,
}

impl CodexProvider {
    pub fn new(cli_path: &str) -> Self {
        Self {
            cli_path: cli_path.to_string(),
        }
    }

    fn base_command(&self, working_dir: &Path, extra_flags: &[String]) -> Command {
        let mut cmd = Command::new(&self.cli_path);
        cmd.current_dir(working_dir);
        for flag in ProviderKind::Codex.autonomous_flags() {
            cmd.arg(flag);
        }
        for flag in extra_flags {
            cmd.arg(flag);
        }
        cmd
    }
}

#[async_trait::async_trait]
impl Provider for CodexProvider {
    fn name(&self) -> &str {
        "codex"
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Codex
    }

    fn is_available(&self) -> Result<bool> {
        Ok(which::which(&self.cli_path).is_ok())
    }

    async fn run(
        &self,
        prompt: &str,
        working_dir: &Path,
        extra_flags: &[String],
    ) -> Result<AgentOutput> {
        let start = Instant::now();
        let mut cmd = self.base_command(working_dir, extra_flags);
        for flag in ProviderKind::Codex.non_interactive_flags() {
            cmd.arg(flag);
        }
        cmd.arg(prompt);
        let output = cmd.output().await?;
        let duration = start.elapsed();

        Ok(AgentOutput {
            text: String::from_utf8_lossy(&output.stdout).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            duration,
        })
    }

    async fn run_interactive(
        &self,
        prompt: &str,
        working_dir: &Path,
        extra_flags: &[String],
    ) -> Result<InteractiveSession> {
        let mut cmd = self.base_command(working_dir, extra_flags);
        cmd.arg(prompt);
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
        let child = cmd.spawn()?;
        Ok(InteractiveSession { child })
    }
}
```

- [ ] **Step 6: Implement detection**

Create `src/provider/detection.rs`:
```rust
use super::traits::ProviderKind;

pub struct DetectedProvider {
    pub kind: ProviderKind,
    pub path: String,
}

pub fn detect_providers() -> Vec<DetectedProvider> {
    let mut found = Vec::new();

    for kind in [ProviderKind::Claude, ProviderKind::Codex] {
        if let Ok(path) = which::which(kind.cli_name()) {
            found.push(DetectedProvider {
                kind,
                path: path.to_string_lossy().to_string(),
            });
        }
    }

    found
}
```

- [ ] **Step 7: Wire up provider module**

Create `src/provider/mod.rs`:
```rust
mod traits;
mod claude;
mod codex;
mod detection;

pub use traits::{AgentOutput, InteractiveSession, Provider, ProviderKind};
pub use claude::ClaudeProvider;
pub use codex::CodexProvider;
pub use detection::{detect_providers, DetectedProvider};

use crate::config::Config;

pub fn create_provider(config: &Config, agent_provider: &str) -> Option<Box<dyn Provider>> {
    let provider_name = if agent_provider == "default" {
        &config.default_provider
    } else {
        agent_provider
    };

    let provider_config = config.providers.get(provider_name)?;

    match provider_name.as_str() {
        "claude" => Some(Box::new(ClaudeProvider::new(&provider_config.cli))),
        "codex" => Some(Box::new(CodexProvider::new(&provider_config.cli))),
        _ => None,
    }
}
```

Update `src/lib.rs`:
```rust
pub mod config;
pub mod state;
pub mod provider;
```

- [ ] **Step 8: Run tests**

Run: `cargo test --test provider_test`
Expected: 3 tests PASS

- [ ] **Step 9: Commit**

```bash
git add src/provider/ src/lib.rs tests/provider_test.rs Cargo.toml
git commit -m "feat: add provider abstraction with Claude and Codex CLI adapters"
```

---

### Task 5: Structured Output Parser

**Files:**
- Create: `src/agent/mod.rs`
- Create: `src/agent/output_parser.rs`
- Create: `src/agent/prompts.rs`
- Create: `tests/output_parser_test.rs`
- Create: `prompts/researcher.md` (placeholder)
- Modify: `src/lib.rs`

- [ ] **Step 1: Write failing tests for output parsing**

Create `tests/output_parser_test.rs`:
```rust
use kora::agent::output_parser::{parse_verdict, parse_review, parse_validation, Verdict, ReviewSummary, ValidationResult};

#[test]
fn test_parse_verdict_approve() {
    let text = r#"
Some reasoning here...

<!-- VERDICT -->
- REVIEWER_FINDING_1: DISMISSED
- REVIEWER_FINDING_2: DISMISSED
- OVERALL: APPROVE
- VALID_COUNT: 0
- DISMISSED_COUNT: 2
<!-- /VERDICT -->
"#;
    let verdict = parse_verdict(text).unwrap();
    assert_eq!(verdict.overall, "APPROVE");
    assert_eq!(verdict.valid_count, 0);
    assert_eq!(verdict.dismissed_count, 2);
}

#[test]
fn test_parse_verdict_revise() {
    let text = r#"
<!-- VERDICT -->
- REVIEWER_FINDING_1: VALID
- SECURITY_FINDING_1: DISMISSED
- OVERALL: REVISE
- VALID_COUNT: 1
- DISMISSED_COUNT: 1
<!-- /VERDICT -->
"#;
    let verdict = parse_verdict(text).unwrap();
    assert_eq!(verdict.overall, "REVISE");
    assert_eq!(verdict.valid_count, 1);
}

#[test]
fn test_parse_verdict_missing_markers_returns_none() {
    let text = "No structured output here";
    assert!(parse_verdict(text).is_none());
}

#[test]
fn test_parse_review_summary() {
    let text = r#"
<!-- REVIEW -->
- FINDING_1: HIGH No migration strategy
- FINDING_2: MEDIUM Missing error boundary
- FINDING_3: LOW Const enum suggestion
- TOTAL: 3 findings (1 high, 1 medium, 1 low)
<!-- /REVIEW -->
"#;
    let review = parse_review(text).unwrap();
    assert_eq!(review.findings.len(), 3);
    assert_eq!(review.findings[0].severity, "HIGH");
}

#[test]
fn test_parse_validation_pass() {
    let text = r#"
<!-- VALIDATION -->
- STATUS: PASS
- BLOCKING_ISSUES: 0
- MINOR_ISSUES: 1
- TEST_SUITE: 42 passed, 0 failed
- TYPE_CHECK: PASS
<!-- /VALIDATION -->
"#;
    let result = parse_validation(text).unwrap();
    assert!(result.passed);
    assert_eq!(result.blocking_issues, 0);
}
```

- [ ] **Step 2: Run tests to verify they don't compile yet**

Run: `cargo test --test output_parser_test 2>&1 || true`
Expected: compilation error — module not found. This is expected.

- [ ] **Step 3: Implement output parser**

Create `src/agent/output_parser.rs`:
```rust
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Verdict {
    pub findings: Vec<FindingVerdict>,
    pub overall: String,
    pub valid_count: u32,
    pub dismissed_count: u32,
}

#[derive(Debug, Clone)]
pub struct FindingVerdict {
    pub id: String,
    pub verdict: String,
}

#[derive(Debug, Clone)]
pub struct ReviewSummary {
    pub findings: Vec<ReviewFinding>,
    pub total: u32,
}

#[derive(Debug, Clone)]
pub struct ReviewFinding {
    pub id: String,
    pub severity: String,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub passed: bool,
    pub blocking_issues: u32,
    pub minor_issues: u32,
    pub tests_passed: u32,
    pub tests_failed: u32,
    pub type_check_passed: bool,
}

fn extract_block(text: &str, open_tag: &str, close_tag: &str) -> Option<String> {
    let start = text.find(open_tag)?;
    let end = text.find(close_tag)?;
    if end <= start {
        return None;
    }
    let content = &text[start + open_tag.len()..end];
    Some(content.trim().to_string())
}

pub fn parse_verdict(text: &str) -> Option<Verdict> {
    let block = extract_block(text, "<!-- VERDICT -->", "<!-- /VERDICT -->")?;
    let mut findings = Vec::new();
    let mut overall = String::new();
    let mut valid_count = 0u32;
    let mut dismissed_count = 0u32;

    for line in block.lines() {
        let line = line.trim().trim_start_matches('-').trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("OVERALL:") {
            overall = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("VALID_COUNT:") {
            valid_count = rest.trim().parse().unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("DISMISSED_COUNT:") {
            dismissed_count = rest.trim().parse().unwrap_or(0);
        } else if let Some((id, verdict)) = line.split_once(':') {
            findings.push(FindingVerdict {
                id: id.trim().to_string(),
                verdict: verdict.trim().to_string(),
            });
        }
    }

    if overall.is_empty() {
        return None;
    }

    Some(Verdict {
        findings,
        overall,
        valid_count,
        dismissed_count,
    })
}

pub fn parse_review(text: &str) -> Option<ReviewSummary> {
    let block = extract_block(text, "<!-- REVIEW -->", "<!-- /REVIEW -->")?;
    let mut findings = Vec::new();
    let mut total = 0u32;

    for line in block.lines() {
        let line = line.trim().trim_start_matches('-').trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("TOTAL:") {
            if let Some(num) = line.split_whitespace().nth(1) {
                total = num.parse().unwrap_or(0);
            }
        } else if let Some((id_part, rest)) = line.split_once(':') {
            let parts: Vec<&str> = rest.trim().splitn(2, ' ').collect();
            if parts.len() == 2 {
                findings.push(ReviewFinding {
                    id: id_part.trim().to_string(),
                    severity: parts[0].to_string(),
                    title: parts[1].to_string(),
                });
            }
        }
    }

    Some(ReviewSummary {
        total: if total > 0 { total } else { findings.len() as u32 },
        findings,
    })
}

pub fn parse_validation(text: &str) -> Option<ValidationResult> {
    let block = extract_block(text, "<!-- VALIDATION -->", "<!-- /VALIDATION -->")?;
    let mut passed = false;
    let mut blocking = 0u32;
    let mut minor = 0u32;
    let mut tests_passed = 0u32;
    let mut tests_failed = 0u32;
    let mut type_check = false;

    for line in block.lines() {
        let line = line.trim().trim_start_matches('-').trim();

        if let Some(rest) = line.strip_prefix("STATUS:") {
            passed = rest.trim().eq_ignore_ascii_case("PASS");
        } else if let Some(rest) = line.strip_prefix("BLOCKING_ISSUES:") {
            blocking = rest.trim().parse().unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("MINOR_ISSUES:") {
            minor = rest.trim().parse().unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("TEST_SUITE:") {
            let parts: Vec<&str> = rest.split(',').collect();
            if let Some(p) = parts.first() {
                tests_passed = p.trim().split_whitespace().next()
                    .and_then(|n| n.parse().ok()).unwrap_or(0);
            }
            if let Some(f) = parts.get(1) {
                tests_failed = f.trim().split_whitespace().next()
                    .and_then(|n| n.parse().ok()).unwrap_or(0);
            }
        } else if let Some(rest) = line.strip_prefix("TYPE_CHECK:") {
            type_check = rest.trim().eq_ignore_ascii_case("PASS");
        }
    }

    Some(ValidationResult {
        passed,
        blocking_issues: blocking,
        minor_issues: minor,
        tests_passed,
        tests_failed,
        type_check_passed: type_check,
    })
}
```

- [ ] **Step 4: Create prompt loader**

Create `src/agent/prompts.rs`:
```rust
pub const RESEARCHER_PROMPT: &str = include_str!("../../prompts/researcher.md");
pub const REVIEWER_PROMPT: &str = include_str!("../../prompts/reviewer.md");
pub const SECURITY_AUDITOR_PROMPT: &str = include_str!("../../prompts/security_auditor.md");
pub const JUDGE_PROMPT: &str = include_str!("../../prompts/judge.md");
pub const PLANNER_PROMPT: &str = include_str!("../../prompts/planner.md");
pub const TEST_ARCHITECT_PROMPT: &str = include_str!("../../prompts/test_architect.md");
pub const IMPLEMENTOR_PROMPT: &str = include_str!("../../prompts/implementor.md");
pub const VALIDATOR_PROMPT: &str = include_str!("../../prompts/validator.md");

pub fn assemble_prompt(base: &str, custom_instructions: Option<&str>, context: &str) -> String {
    let mut prompt = base.to_string();

    if let Some(custom) = custom_instructions {
        prompt.push_str("\n\n---\n\n## Additional Instructions\n\n");
        prompt.push_str(custom);
    }

    prompt.push_str("\n\n---\n\n");
    prompt.push_str(context);

    prompt
}
```

Create placeholder prompt files (content from the spec will be filled in Phase 2):

```bash
mkdir -p prompts
```

Create `prompts/researcher.md`, `prompts/reviewer.md`, `prompts/security_auditor.md`,
`prompts/judge.md`, `prompts/planner.md`, `prompts/test_architect.md`,
`prompts/implementor.md`, `prompts/validator.md` — each with placeholder text:
```
Placeholder — full prompt will be added in Phase 2.
```

- [ ] **Step 5: Wire up agent module**

Create `src/agent/mod.rs`:
```rust
pub mod output_parser;
pub mod prompts;
```

Update `src/lib.rs`:
```rust
pub mod config;
pub mod state;
pub mod provider;
pub mod agent;
```

- [ ] **Step 6: Run tests**

Run: `cargo test --test output_parser_test`
Expected: 5 tests PASS

- [ ] **Step 7: Commit**

```bash
git add src/agent/ prompts/ tests/output_parser_test.rs src/lib.rs
git commit -m "feat: add agent prompt system and structured output parser"
```

---

### Task 6: Terminal UI Foundation

**Files:**
- Create: `src/terminal/mod.rs`
- Create: `src/terminal/renderer.rs`
- Create: `src/terminal/verbosity.rs`
- Create: `src/terminal/selector.rs`
- Create: `src/terminal/input.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Implement verbosity mode**

Create `src/terminal/verbosity.rs`:
```rust
use crate::config::Verbosity;

pub struct VerbosityState {
    current: Verbosity,
}

impl VerbosityState {
    pub fn new(default: Verbosity) -> Self {
        Self { current: default }
    }

    pub fn current(&self) -> Verbosity {
        self.current
    }

    pub fn cycle(&mut self) -> Verbosity {
        self.current = match self.current {
            Verbosity::Focused => Verbosity::Detailed,
            Verbosity::Detailed => Verbosity::Verbose,
            Verbosity::Verbose => Verbosity::Focused,
        };
        self.current
    }

    pub fn label(&self) -> &'static str {
        match self.current {
            Verbosity::Focused => "focused",
            Verbosity::Detailed => "detailed",
            Verbosity::Verbose => "verbose",
        }
    }
}
```

- [ ] **Step 2: Implement terminal renderer**

Create `src/terminal/renderer.rs`:
```rust
use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Attribute, SetAttribute},
    cursor,
    terminal,
};
use std::io::{self, Write};

use crate::config::Verbosity;

pub struct Renderer {
    stdout: io::Stdout,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            stdout: io::stdout(),
        }
    }

    pub fn stage_header(&mut self, name: &str, status: &str) {
        let dots: String = std::iter::repeat('·')
            .take(50usize.saturating_sub(name.len() + status.len()))
            .collect();

        execute!(
            self.stdout,
            Print("\n  "),
            SetForegroundColor(Color::White),
            SetAttribute(Attribute::Bold),
            Print(name),
            SetAttribute(Attribute::Reset),
            Print(" "),
            SetForegroundColor(Color::DarkGrey),
            Print(dots),
            Print(" "),
            ResetColor,
            Print(status),
            Print(" "),
            SetForegroundColor(Color::Green),
            Print("●"),
            ResetColor,
            Print("\n"),
        )
        .ok();
    }

    pub fn stage_complete(&mut self, name: &str, duration_secs: u64) {
        execute!(
            self.stdout,
            Print("\n  "),
            SetForegroundColor(Color::Green),
            Print("✓ "),
            ResetColor,
            Print(name),
            SetForegroundColor(Color::DarkGrey),
            Print(format!("  {}s", duration_secs)),
            ResetColor,
            Print("\n"),
        )
        .ok();
    }

    pub fn finding(&mut self, severity: &str, text: &str) {
        let (glyph, color) = match severity.to_uppercase().as_str() {
            "HIGH" => ("▲", Color::Red),
            "MEDIUM" | "MED" => ("■", Color::Yellow),
            _ => ("·", Color::DarkGrey),
        };

        execute!(
            self.stdout,
            Print("    "),
            SetForegroundColor(color),
            Print(glyph),
            Print(" "),
            Print(severity.to_uppercase()),
            ResetColor,
            Print(format!("   {}\n", text)),
        )
        .ok();
    }

    pub fn verdict_line(&mut self, title: &str, accepted: bool, reason: &str) {
        let (glyph, color) = if accepted {
            ("▲", Color::Red)
        } else {
            ("·", Color::DarkGrey)
        };

        let status = if accepted { "accepted" } else { "dismissed" };

        execute!(
            self.stdout,
            Print("    "),
            SetForegroundColor(color),
            Print(glyph),
            ResetColor,
            Print(format!(" {:<25} {} — {}\n", title, status, reason)),
        )
        .ok();
    }

    pub fn separator(&mut self) {
        execute!(
            self.stdout,
            SetForegroundColor(Color::DarkGrey),
            Print("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"),
            ResetColor,
        )
        .ok();
    }

    pub fn info(&mut self, text: &str) {
        execute!(
            self.stdout,
            SetForegroundColor(Color::DarkGrey),
            Print(format!("  {}\n", text)),
            ResetColor,
        )
        .ok();
    }

    pub fn text(&mut self, text: &str) {
        execute!(
            self.stdout,
            Print(format!("  {}\n", text)),
        )
        .ok();
    }

    pub fn welcome(&mut self, version: &str, provider: &str, checkpoints: usize) {
        execute!(
            self.stdout,
            Print("\n"),
            SetForegroundColor(Color::White),
            SetAttribute(Attribute::Bold),
            Print(format!("  kora v{}", version)),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::DarkGrey),
            Print(format!(" · {} (default) · {} checkpoints configured", provider, checkpoints)),
            ResetColor,
            Print("\n\n"),
            Print("  ready. describe what you'd like to build, fix, or change.\n\n"),
        )
        .ok();
    }
}
```

- [ ] **Step 3: Implement arrow-key selector**

Create `src/terminal/selector.rs`:
```rust
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal,
};
use std::io::{self, Write};

pub fn select(prompt: &str, options: &[&str]) -> io::Result<usize> {
    let mut stdout = io::stdout();
    let mut selected = 0usize;

    execute!(stdout, Print(format!("\n  ? {}\n\n", prompt)))?;

    terminal::enable_raw_mode()?;

    loop {
        // Render options
        for (i, option) in options.iter().enumerate() {
            execute!(stdout, cursor::MoveToColumn(0))?;
            if i == selected {
                execute!(
                    stdout,
                    SetForegroundColor(Color::Cyan),
                    Print(format!("    ❯ {}\n", option)),
                    ResetColor,
                )?;
            } else {
                execute!(stdout, Print(format!("      {}\n", option)))?;
            }
        }

        execute!(
            stdout,
            Print("\n"),
            SetForegroundColor(Color::DarkGrey),
            Print("                                          ↑↓ navigate · enter select"),
            ResetColor,
        )?;

        stdout.flush()?;

        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Up => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                KeyCode::Down => {
                    if selected < options.len() - 1 {
                        selected += 1;
                    }
                }
                KeyCode::Enter => break,
                KeyCode::Char('q') | KeyCode::Esc => {
                    terminal::disable_raw_mode()?;
                    return Ok(selected);
                }
                _ => {}
            }
        }

        // Move cursor back up to redraw
        let lines_to_clear = options.len() + 2; // options + hint line + blank
        execute!(
            stdout,
            cursor::MoveUp(lines_to_clear as u16),
        )?;
    }

    terminal::disable_raw_mode()?;
    execute!(stdout, Print("\n"))?;
    Ok(selected)
}

pub fn multi_select(prompt: &str, options: &[&str]) -> io::Result<Vec<usize>> {
    let mut stdout = io::stdout();
    let mut selected = 0usize;
    let mut toggled = vec![false; options.len()];

    execute!(stdout, Print(format!("\n  ? {}\n\n", prompt)))?;

    terminal::enable_raw_mode()?;

    loop {
        for (i, option) in options.iter().enumerate() {
            execute!(stdout, cursor::MoveToColumn(0))?;
            let marker = if toggled[i] { "◉" } else { "◯" };
            if i == selected {
                execute!(
                    stdout,
                    SetForegroundColor(Color::Cyan),
                    Print(format!("    {} {}\n", marker, option)),
                    ResetColor,
                )?;
            } else {
                execute!(stdout, Print(format!("    {} {}\n", marker, option)))?;
            }
        }

        execute!(
            stdout,
            Print("\n"),
            SetForegroundColor(Color::DarkGrey),
            Print("                              ↑↓ navigate · space toggle · enter confirm"),
            ResetColor,
        )?;

        stdout.flush()?;

        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Up => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                KeyCode::Down => {
                    if selected < options.len() - 1 {
                        selected += 1;
                    }
                }
                KeyCode::Char(' ') => {
                    toggled[selected] = !toggled[selected];
                }
                KeyCode::Enter => break,
                KeyCode::Esc => {
                    terminal::disable_raw_mode()?;
                    return Ok(vec![]);
                }
                _ => {}
            }
        }

        let lines_to_clear = options.len() + 2;
        execute!(stdout, cursor::MoveUp(lines_to_clear as u16))?;
    }

    terminal::disable_raw_mode()?;
    execute!(stdout, Print("\n"))?;

    Ok(toggled
        .iter()
        .enumerate()
        .filter(|(_, t)| **t)
        .map(|(i, _)| i)
        .collect())
}
```

- [ ] **Step 4: Implement text input**

Create `src/terminal/input.rs`:
```rust
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal,
};
use std::io::{self, Write};

pub fn read_line(prompt: &str) -> io::Result<String> {
    let mut stdout = io::stdout();

    execute!(
        stdout,
        SetForegroundColor(Color::Cyan),
        Print(prompt),
        ResetColor,
    )?;
    stdout.flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

pub fn read_user_input() -> io::Result<String> {
    read_line("> ")
}
```

- [ ] **Step 5: Wire up terminal module**

Create `src/terminal/mod.rs`:
```rust
pub mod renderer;
pub mod selector;
pub mod input;
pub mod verbosity;

pub use renderer::Renderer;
pub use verbosity::VerbosityState;
```

Update `src/lib.rs`:
```rust
pub mod config;
pub mod state;
pub mod provider;
pub mod agent;
pub mod terminal;
```

- [ ] **Step 6: Write tests for verbosity and selector logic**

Create `tests/verbosity_test.rs`:
```rust
use kora::config::Verbosity;
use kora::terminal::VerbosityState;

#[test]
fn test_verbosity_starts_at_default() {
    let state = VerbosityState::new(Verbosity::Focused);
    assert_eq!(state.current(), Verbosity::Focused);
    assert_eq!(state.label(), "focused");
}

#[test]
fn test_verbosity_cycles_through_modes() {
    let mut state = VerbosityState::new(Verbosity::Focused);

    assert_eq!(state.cycle(), Verbosity::Detailed);
    assert_eq!(state.label(), "detailed");

    assert_eq!(state.cycle(), Verbosity::Verbose);
    assert_eq!(state.label(), "verbose");

    assert_eq!(state.cycle(), Verbosity::Focused);
    assert_eq!(state.label(), "focused");
}
```

- [ ] **Step 7: Run tests**

Run: `cargo test --test verbosity_test`
Expected: 2 tests PASS

- [ ] **Step 8: Verify full compilation**

Run: `cargo build`
Expected: compiles with no errors

- [ ] **Step 9: Commit**

```bash
git add src/terminal/ src/lib.rs tests/verbosity_test.rs
git commit -m "feat: add terminal UI with renderer, arrow-key selector, and verbosity modes"
```

---

### Task 7: CLI App with Clap

**Files:**
- Create: `src/cli/mod.rs`
- Create: `src/cli/app.rs`
- Create: `src/cli/configure.rs`
- Modify: `src/main.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Define CLI structure with clap**

Create `src/cli/app.rs`:
```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "kora", version, about = "Multi-agent development orchestration CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start a one-shot run with a specific request
    Run {
        /// The request to execute
        request: String,

        /// Override default provider
        #[arg(short, long)]
        provider: Option<String>,

        /// No checkpoints, full autopilot
        #[arg(long)]
        yolo: bool,

        /// Checkpoints at every stage
        #[arg(long)]
        careful: bool,

        /// Research + review only, no implementation
        #[arg(long)]
        dry_run: bool,
    },

    /// Interactive setup wizard
    Configure,

    /// Resume an interrupted session
    Resume,

    /// View past runs
    History,

    /// Clean up old run data
    Clean,
}
```

- [ ] **Step 2: Implement configure wizard**

Create `src/cli/configure.rs`:
```rust
use anyhow::Result;
use std::path::Path;

use crate::config::{self, Config, BranchStrategy, Verbosity};
use crate::provider::detect_providers;
use crate::state::Checkpoint;
use crate::terminal::selector;

pub fn run_configure(project_root: &Path) -> Result<()> {
    let mut config = config::load(project_root)?;

    let detected = detect_providers();
    if detected.is_empty() {
        eprintln!("  No AI CLI tools detected. Install claude or codex first.");
        return Ok(());
    }

    let provider_names: Vec<&str> = detected.iter().map(|p| p.kind.cli_name()).collect();

    let default_idx = selector::select(
        "Default AI provider:",
        &provider_names,
    )?;
    config.default_provider = provider_names[default_idx].to_string();

    let assign_idx = selector::select(
        "Assign providers per agent role, or use default for all?",
        &["Use default for all", "Assign per role"],
    )?;

    if assign_idx == 1 {
        let roles = [
            "researcher", "reviewer", "security_auditor", "judge",
            "planner", "test_architect", "implementor", "validator",
        ];
        let mut options: Vec<&str> = vec!["default"];
        options.extend(&provider_names);

        for role in roles {
            let idx = selector::select(
                &format!("Provider for {}:", role),
                &options,
            )?;
            let provider = options[idx].to_string();
            match role {
                "researcher" => config.agents.researcher.provider = provider,
                "reviewer" => config.agents.reviewer.provider = provider,
                "security_auditor" => config.agents.security_auditor.provider = provider,
                "judge" => config.agents.judge.provider = provider,
                "planner" => config.agents.planner.provider = provider,
                "test_architect" => config.agents.test_architect.provider = provider,
                "implementor" => config.agents.implementor.provider = provider,
                "validator" => config.agents.validator.provider = provider,
                _ => {}
            }
        }
    }

    let checkpoint_options = [
        "After researcher proposes direction",
        "After each review/judge iteration",
        "After planner breaks down tasks",
        "After implementation completes",
    ];
    let checkpoint_selected = selector::multi_select(
        "Which stages require your approval before proceeding?",
        &checkpoint_options,
    )?;

    config.checkpoints = checkpoint_selected
        .iter()
        .filter_map(|&i| match i {
            0 => Some(Checkpoint::AfterResearcher),
            1 => Some(Checkpoint::AfterReviewLoop),
            2 => Some(Checkpoint::AfterPlanner),
            3 => Some(Checkpoint::AfterImplementation),
            _ => None,
        })
        .collect();

    let branch_idx = selector::select(
        "Default branch strategy for implementation:",
        &["Separate branch per task", "Single feature branch", "Let planner decide"],
    )?;
    config.implementation.branch_strategy = match branch_idx {
        0 => BranchStrategy::Separate,
        1 => BranchStrategy::Single,
        _ => BranchStrategy::PlannerDecides,
    };

    let verbosity_idx = selector::select(
        "Default output verbosity:",
        &["focused", "detailed", "verbose"],
    )?;
    config.output.default_verbosity = match verbosity_idx {
        0 => Verbosity::Focused,
        1 => Verbosity::Detailed,
        _ => Verbosity::Verbose,
    };

    config::save(project_root, &config)?;
    println!("\n  ✅ Configuration saved to .kora/config.yml\n");

    Ok(())
}
```

- [ ] **Step 3: Wire up CLI module**

Create `src/cli/mod.rs`:
```rust
pub mod app;
pub mod configure;
```

Update `src/lib.rs`:
```rust
pub mod config;
pub mod state;
pub mod provider;
pub mod agent;
pub mod terminal;
pub mod cli;
```

- [ ] **Step 4: Implement main.rs**

Replace `src/main.rs`:
```rust
use anyhow::Result;
use clap::Parser;
use std::env;

use kora::cli::app::{Cli, Commands};
use kora::cli::configure;
use kora::config;
use kora::provider::detect_providers;
use kora::terminal::Renderer;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let project_root = env::current_dir()?;

    match cli.command {
        Some(Commands::Configure) => {
            configure::run_configure(&project_root)?;
        }
        Some(Commands::Run { request, .. }) => {
            println!("  run not yet implemented: {}", request);
        }
        Some(Commands::Resume) => {
            println!("  resume not yet implemented");
        }
        Some(Commands::History) => {
            println!("  history not yet implemented");
        }
        Some(Commands::Clean) => {
            println!("  clean not yet implemented");
        }
        None => {
            let config = config::load(&project_root)?;
            let detected = detect_providers();
            let mut renderer = Renderer::new();

            if detected.is_empty() {
                eprintln!("  No AI CLI tools detected. Install claude or codex first.");
                eprintln!("  Run `kora configure` after installing a provider.");
                return Ok(());
            }

            renderer.welcome(
                env!("CARGO_PKG_VERSION"),
                &config.default_provider,
                config.checkpoints.len(),
            );

            // Phase 2: interactive session loop will go here
            let input = kora::terminal::input::read_user_input()?;
            if input.is_empty() {
                return Ok(());
            }

            println!("\n  received: \"{}\"", input);
            println!("  pipeline not yet implemented — coming in Phase 2\n");
        }
    }

    Ok(())
}
```

- [ ] **Step 5: Verify the CLI works**

Run: `cargo run`
Expected: shows welcome message with version, provider, checkpoints, and `>` prompt

Run: `cargo run -- configure`
Expected: interactive wizard starts

Run: `cargo run -- --help`
Expected: shows help text with all subcommands

- [ ] **Step 6: Commit**

```bash
git add src/cli/ src/main.rs src/lib.rs
git commit -m "feat: add CLI with clap, configure wizard, and interactive session entry"
```

---

### Task 8: Install Script & CI Release Pipeline

**Files:**
- Create: `install.sh`
- Create: `.github/workflows/release.yml`
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create install script**

Create `install.sh`:
```bash
#!/bin/bash
set -euo pipefail

REPO="kora-ai/kora"
BINARY="kora"

get_arch() {
    local arch
    arch=$(uname -m)
    case "$arch" in
        x86_64|amd64) echo "x86_64" ;;
        arm64|aarch64) echo "aarch64" ;;
        *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
    esac
}

get_os() {
    local os
    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    case "$os" in
        linux) echo "unknown-linux-gnu" ;;
        darwin) echo "apple-darwin" ;;
        *) echo "Unsupported OS: $os" >&2; exit 1 ;;
    esac
}

main() {
    local arch os target version download_url tmp_dir

    arch=$(get_arch)
    os=$(get_os)
    target="${arch}-${os}"

    echo "Detecting platform: ${target}"

    version=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' | head -1 | cut -d'"' -f4)

    if [ -z "$version" ]; then
        echo "Failed to fetch latest version" >&2
        exit 1
    fi

    echo "Installing kora ${version}..."

    download_url="https://github.com/${REPO}/releases/download/${version}/kora-${target}.tar.gz"

    tmp_dir=$(mktemp -d)
    trap 'rm -rf "$tmp_dir"' EXIT

    curl -fsSL "$download_url" | tar xz -C "$tmp_dir"

    local install_dir="/usr/local/bin"
    if [ ! -w "$install_dir" ]; then
        echo "Installing to ${install_dir} (requires sudo)..."
        sudo mv "${tmp_dir}/${BINARY}" "${install_dir}/${BINARY}"
    else
        mv "${tmp_dir}/${BINARY}" "${install_dir}/${BINARY}"
    fi

    chmod +x "${install_dir}/${BINARY}"

    echo "✓ kora ${version} installed to ${install_dir}/${BINARY}"
}

main
```

- [ ] **Step 2: Create CI workflow**

Create `.github/workflows/ci.yml`:
```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --all
      - run: cargo clippy -- -D warnings
      - run: cargo fmt --check
```

- [ ] **Step 3: Create release workflow**

Create `.github/workflows/release.yml`:
```yaml
name: Release

on:
  push:
    tags: ["v*"]

permissions:
  contents: write

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: aarch64-apple-darwin
            os: macos-latest

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install cross-compilation tools
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}
        env:
          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER: aarch64-linux-gnu-gcc

      - name: Package
        run: |
          cd target/${{ matrix.target }}/release
          tar czf ../../../kora-${{ matrix.target }}.tar.gz kora
          cd ../../..

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: kora-${{ matrix.target }}
          path: kora-${{ matrix.target }}.tar.gz

  release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4

      - name: Create release
        uses: softprops/action-gh-release@v2
        with:
          files: |
            kora-*/kora-*.tar.gz
          generate_release_notes: true

  publish-crate:
    needs: release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo publish
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

- [ ] **Step 4: Create npm package for distribution**

Create `npm/package.json`:
```json
{
  "name": "kora-ai",
  "version": "0.1.0",
  "description": "Multi-agent development orchestration CLI",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/kora-ai/kora"
  },
  "bin": {
    "kora": "bin/kora"
  },
  "scripts": {
    "postinstall": "node install.js"
  },
  "files": [
    "bin/",
    "install.js"
  ],
  "os": ["darwin", "linux"],
  "cpu": ["x64", "arm64"]
}
```

Create `npm/install.js`:
```javascript
const { execFileSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const REPO = "kora-ai/kora";
const BIN_DIR = path.join(__dirname, "bin");

const PLATFORM_MAP = {
  "darwin-arm64": "aarch64-apple-darwin",
  "darwin-x64": "x86_64-apple-darwin",
  "linux-arm64": "aarch64-unknown-linux-gnu",
  "linux-x64": "x86_64-unknown-linux-gnu",
};

function install() {
  const platform = `${process.platform}-${process.arch}`;
  const target = PLATFORM_MAP[platform];

  if (!target) {
    console.error(`Unsupported platform: ${platform}`);
    process.exit(1);
  }

  const version = require("./package.json").version;
  const url = `https://github.com/${REPO}/releases/download/v${version}/kora-${target}.tar.gz`;

  console.log(`Downloading kora v${version} for ${target}...`);

  fs.mkdirSync(BIN_DIR, { recursive: true });

  const tmpFile = path.join(BIN_DIR, "kora.tar.gz");
  execFileSync("curl", ["-fsSL", "-o", tmpFile, url], { stdio: "inherit" });
  execFileSync("tar", ["xzf", tmpFile, "-C", BIN_DIR], { stdio: "inherit" });
  fs.unlinkSync(tmpFile);
  fs.chmodSync(path.join(BIN_DIR, "kora"), 0o755);

  console.log("✓ kora installed successfully");
}

install();
```

Create `npm/bin/` directory:
```bash
mkdir -p npm/bin && touch npm/bin/.gitkeep
```

- [ ] **Step 5: Add npm publish to release workflow**

Add to `.github/workflows/release.yml` after `publish-crate` job:
```yaml
  publish-npm:
    needs: release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          registry-url: https://registry.npmjs.org
      - name: Update npm package version
        run: |
          VERSION=${GITHUB_REF_NAME#v}
          cd npm
          node -e "const p=require('./package.json'); p.version='${VERSION}'; require('fs').writeFileSync('package.json', JSON.stringify(p, null, 2))"
      - name: Publish to npm
        run: cd npm && npm publish
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

- [ ] **Step 6: Make install.sh executable**

```bash
chmod +x install.sh
```

- [ ] **Step 7: Commit**

```bash
git add install.sh npm/ .github/
git commit -m "feat: add install script, npm package, and CI/release workflows"
```

---

### Task 9: Integration Test — Full CLI Smoke Test

**Files:**
- Create: `tests/cli_test.rs`

- [ ] **Step 1: Write CLI integration tests**

Create `tests/cli_test.rs`:
```rust
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_kora_help() {
    Command::cargo_bin("kora")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Multi-agent development orchestration CLI"));
}

#[test]
fn test_kora_version() {
    Command::cargo_bin("kora")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("kora"));
}

#[test]
fn test_kora_run_placeholder() {
    Command::cargo_bin("kora")
        .unwrap()
        .args(["run", "test request"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not yet implemented"));
}
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test --test cli_test`
Expected: 3 tests PASS

- [ ] **Step 3: Run full test suite**

Run: `cargo test`
Expected: all tests PASS (config, state, provider, output_parser, cli)

- [ ] **Step 4: Run clippy and fmt**

Run: `cargo clippy -- -D warnings && cargo fmt --check`
Expected: no warnings, no formatting issues. If fmt fails, run `cargo fmt` first.

- [ ] **Step 5: Commit**

```bash
git add tests/cli_test.rs
git commit -m "test: add CLI integration smoke tests"
```

---

## Phase 1 Completion Checklist

After completing all tasks, verify:

- [ ] `cargo run` → shows welcome, prompts for input
- [ ] `cargo run -- configure` → interactive wizard works
- [ ] `cargo run -- --help` → shows all subcommands
- [ ] `cargo run -- run "test"` → placeholder message
- [ ] `cargo test` → all tests pass
- [ ] `cargo clippy -- -D warnings` → no warnings
- [ ] `.kora/config.yml` is created after configure
- [ ] `.kora/.gitignore` ignores `runs/`

## What's Next

**Phase 2: Core Pipeline** — implements the researcher (interactive session), reviewer, security auditor, judge, and the review loop. After Phase 2, a user can run `kora`, describe what they want, and get a reviewed plan.

**Phase 3: Planning & Implementation** — planner, test architect, implementor fleet with parallel execution in worktrees, implementation dashboard.

**Phase 4: Validation & Polish** — validator, merge flow, resume, history, clean.
