mod schema;

pub use schema::*;

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const CONFIG_DIR: &str = ".kora";
const CONFIG_FILE: &str = "config.yml";

pub fn home_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(CONFIG_DIR)
}

pub fn home_config_path() -> PathBuf {
    home_dir().join(CONFIG_FILE)
}

pub fn project_config_path(project_root: &Path) -> PathBuf {
    project_root.join(CONFIG_DIR).join(CONFIG_FILE)
}

pub fn runs_dir() -> PathBuf {
    home_dir().join("runs")
}

pub fn load(project_root: &Path) -> Result<Config> {
    let project_path = project_config_path(project_root);
    let home_path = home_config_path();

    let project_config = load_file(&project_path);
    let home_config = load_file(&home_path);

    match (project_config, home_config) {
        (Ok(project), Ok(home)) => Ok(merge_configs(project, home)),
        (Ok(project), Err(_)) => Ok(project),
        (Err(_), Ok(home)) => Ok(home),
        (Err(_), Err(_)) => Ok(Config::default()),
    }
}

fn load_file(path: &Path) -> Result<Config> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config at {}", path.display()))?;
    let config: Config = serde_yaml::from_str(&contents)
        .with_context(|| format!("failed to parse config at {}", path.display()))?;
    Ok(config)
}

fn merge_configs(project: Config, home: Config) -> Config {
    let default = Config::default();

    Config {
        version: home.version,
        default_provider: if home.default_provider != default.default_provider {
            home.default_provider
        } else {
            project.default_provider
        },
        providers: {
            let mut merged = project.providers;
            merged.extend(home.providers);
            merged
        },
        agents: if home.agents != default.agents {
            home.agents
        } else {
            project.agents
        },
        checkpoints: if home.checkpoints != default.checkpoints {
            home.checkpoints
        } else {
            project.checkpoints
        },
        review_loop: if home.review_loop != default.review_loop {
            home.review_loop
        } else {
            project.review_loop
        },
        validation_loop: if home.validation_loop != default.validation_loop {
            home.validation_loop
        } else {
            project.validation_loop
        },
        implementation: if home.implementation != default.implementation {
            home.implementation
        } else {
            project.implementation
        },
        output: if home.output != default.output {
            home.output
        } else {
            project.output
        },
    }
}

pub fn has_user_config(project_root: &Path) -> bool {
    home_config_path().exists() || project_config_path(project_root).exists()
}

pub fn save_home(config: &Config) -> Result<()> {
    let dir = home_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(CONFIG_FILE);
    let yaml = serde_yaml::to_string(config)?;
    std::fs::write(&path, yaml)?;
    Ok(())
}

pub fn save_project(project_root: &Path, config: &Config) -> Result<()> {
    let dir = project_root.join(CONFIG_DIR);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(CONFIG_FILE);
    let yaml = serde_yaml::to_string(config)?;
    std::fs::write(&path, yaml)?;
    Ok(())
}
