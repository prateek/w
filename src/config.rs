use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WorktrunkConfig {
    #[serde(rename = "worktree-path")]
    pub worktree_path: String,
}

impl Default for WorktrunkConfig {
    fn default() -> Self {
        Self {
            worktree_path: "{repo}.{branch}".to_string(),
        }
    }
}

pub fn load_config() -> Result<WorktrunkConfig, confy::ConfyError> {
    confy::load("worktrunk", None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WorktrunkConfig::default();
        assert_eq!(config.worktree_path, "{repo}.{branch}");
    }

    #[test]
    fn test_config_serialization() {
        let config = WorktrunkConfig::default();
        let toml = toml::to_string(&config).unwrap();
        assert!(toml.contains("worktree-path"));
        assert!(toml.contains("{repo}.{branch}"));
    }
}
