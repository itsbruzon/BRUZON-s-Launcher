use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Persistent definition of a Minecraft installation/profile.
///
/// An instance deliberately contains no account identity. The account used for
/// launching is resolved separately from the account manager at launch time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default = "default_ram")]
    pub ram_mb: u32,
    #[serde(default)]
    pub is_fabric: bool,
    #[serde(default)]
    pub playtime_seconds: u64,
    #[serde(default)]
    pub last_launch: Option<u64>,
}

fn default_ram() -> u32 {
    4096
}

impl Instance {
    pub fn new(name: String, version: String, ram_mb: u32, is_fabric: bool) -> Self {
        let id = make_instance_id(&name, &version, is_fabric);
        Self {
            id,
            name,
            version,
            ram_mb,
            is_fabric,
            playtime_seconds: 0,
            last_launch: None,
        }
    }

    /// Returns the private working directory for this instance.
    pub fn game_dir(&self, instances_root: &Path) -> PathBuf {
        instances_root.join(&self.id)
    }
}

/// Produce a stable filesystem-safe identifier from instance metadata.
pub fn make_instance_id(name: &str, version: &str, is_fabric: bool) -> String {
    let name = sanitize_component(name);
    let version = sanitize_component(version);
    let loader = if is_fabric { "fabric" } else { "vanilla" };

    format!("{}-{}-{}", name, version, loader)
}

fn sanitize_component(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            output.push(ch);
        } else {
            output.push('-');
        }
    }

    let output = output.trim_matches('-').to_string();
    if output.is_empty() {
        "instance".to_string()
    } else {
        output
    }
}

/// Ensure an instance directory exists before Minecraft is started.
pub async fn ensure_instance_dir(instance: &Instance, instances_root: &Path) -> Result<PathBuf> {
    let dir = instance.game_dir(instances_root);
    tokio::fs::create_dir_all(&dir).await?;
    Ok(dir)
}

/// Resolve an instance directory while rejecting paths that escape the
/// launcher-managed instances directory.
pub fn validate_instance_dir(instance: &Instance, instances_root: &Path) -> Result<PathBuf> {
    let dir = instance.game_dir(instances_root);
    if dir.parent() != Some(instances_root) {
        return Err(anyhow!("Invalid instance directory"));
    }
    Ok(dir)
}
