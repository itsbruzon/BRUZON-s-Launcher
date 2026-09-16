use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

use crate::models::Theme;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: Theme,
    pub hide_logs: bool,
    pub sidebar_collapsed: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            hide_logs: false,
            sidebar_collapsed: false,
        }
    }
}

impl Settings {
    pub async fn load(config_dir: &Path) -> Self {
        let path = config_dir.join("settings.json");
        match fs::read_to_string(&path).await {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub async fn save(&self, config_dir: &Path) -> Result<(), std::io::Error> {
        let path = config_dir.join("settings.json");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(std::io::Error::other)?;
        fs::write(path, json).await
    }
}
