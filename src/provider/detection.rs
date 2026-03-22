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
