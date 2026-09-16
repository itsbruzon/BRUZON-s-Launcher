use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

#[derive(Clone, Default)]
pub struct JavaManager;

impl JavaManager {
    pub fn new() -> Self {
        Self
    }

    pub fn find_java(&self) -> Result<PathBuf> {
        let mut seen_paths = std::collections::HashSet::new();

        for candidate in Self::java_candidates() {
            if !candidate.exists() {
                continue;
            }

            let path_abs = match std::fs::canonicalize(&candidate) {
                Ok(path) => path,
                Err(_) => continue,
            };

            if !seen_paths.insert(path_abs.clone()) {
                continue;
            }

            if Self::java_runnable(&path_abs) {
                return Ok(path_abs);
            }
        }

        Err(anyhow!(
            "Could not find a Java runtime. Install Java and ensure it is on your PATH or set JAVA_HOME."
        ))
    }

    fn java_runnable(path: &Path) -> bool {
        StdCommand::new(path)
            .arg("-version")
            .output()
            .map(|output| {
                !output.stderr.is_empty() || !output.stdout.is_empty() || output.status.success()
            })
            .unwrap_or(false)
    }

    fn java_candidates() -> Vec<PathBuf> {
        let mut candidates = Vec::new();

        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            candidates.push(PathBuf::from(java_home).join("bin").join("java"));
        }

        let common_paths = [
            "java",
            "/usr/bin/java",
            "/usr/local/bin/java",
            "/opt/java/bin/java",
            "/usr/lib/jvm/java-8-openjdk-amd64/bin/java",
            "/usr/lib/jvm/java-11-openjdk-amd64/bin/java",
            "/usr/lib/jvm/java-17-openjdk-amd64/bin/java",
            "/usr/lib/jvm/java-21-openjdk-amd64/bin/java",
            "/usr/lib/jvm/java-8-openjdk/bin/java",
            "/usr/lib/jvm/java-17-openjdk/bin/java",
            "/usr/lib/jvm/default/bin/java",
        ];

        for path_str in common_paths {
            if path_str.contains('/') {
                candidates.push(PathBuf::from(path_str));
                continue;
            }

            if let Ok(output) = StdCommand::new("which").arg(path_str).output() {
                if output.status.success() {
                    let resolved = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !resolved.is_empty() {
                        candidates.push(PathBuf::from(resolved));
                    }
                }
            }
        }

        if let Ok(entries) = std::fs::read_dir("/usr/lib/jvm") {
            for entry in entries.flatten() {
                candidates.push(entry.path().join("bin").join("java"));
            }
        }

        candidates
    }
}
