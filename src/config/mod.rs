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
