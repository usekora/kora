use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

use super::{PipelineProfile, Stage};

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
    #[serde(default)]
    pub pipeline_profile: Option<PipelineProfile>,
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
            pipeline_profile: None,
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

    pub fn set_error(&mut self, err: &str) {
        self.error = Some(err.to_string());
        self.status = Stage::Failed(err.to_string());
        self.updated_at = Utc::now();
    }

    pub fn increment_iteration(&mut self) {
        self.current_iteration += 1;
        self.updated_at = Utc::now();
    }
}
