use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

#[derive(Clone)]
pub struct JavaManager {
    runtimes_dir: PathBuf,
}

impl JavaManager {
    pub fn new(runtimes_dir: PathBuf) -> Self {
        Self { runtimes_dir }
    }

    fn get_java_version(&self, path: &Path) -> Result<u32> {
        let output = StdCommand::new(path).arg("-version").output()?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        // Example output: openjdk version "17.0.1" ...
        // or: java version "1.8.0_..."

        for line in stderr.lines() {
            if line.contains("version") {
                let parts: Vec<&str> = line.split('"').collect();
                if parts.len() >= 2 {
                    let version_str = parts[1];
                    if version_str.starts_with("1.") {
                        // 1.8.0 -> 8
                        if let Some(minor) = version_str.split('.').nth(1) {
                            if let Ok(v) = minor.parse::<u32>() {
                                return Ok(v);
                            }
                        }
                    } else {
                        // 17.0.1 -> 17
                        if let Some(major) = version_str.split('.').next() {
                            if let Ok(v) = major.parse::<u32>() {
                                return Ok(v);
                            }
                        }
                    }
                }
            }
        }
        anyhow::bail!("Could not parse Java version")
    }

    pub fn find_java(&self, required_version: Option<u32>) -> Result<PathBuf> {
        let mut seen_paths = std::collections::HashSet::new();
        let mut found_versions = Vec::new();

        for candidate in self.java_candidates() {
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

            let version = match self.get_java_version(&path_abs) {
                Ok(version) => version,
                Err(_) => continue,
            };

            found_versions.push(format!("Java {} ({})", version, path_abs.display()));

            if required_version.is_none_or(|req| req == version) {
                return Ok(path_abs);
            }
        }

        if let Some(req) = required_version {
            if found_versions.is_empty() {
                return Err(anyhow!(
                    "Java Runtime {} is missing. Install it manually and try again.",
                    req
                ));
            }

            return Err(anyhow!(
                "Java Runtime {} is missing. Found: {}",
                req,
                found_versions.join(", ")
            ));
        }

        Err(anyhow!("Could not find any installed Java runtime"))
    }

    fn java_candidates(&self) -> Vec<PathBuf> {
        let mut candidates = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.runtimes_dir) {
            for entry in entries.flatten() {
                let path = entry.path().join("bin").join("java");
                candidates.push(path);
            }
        }

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

        if let Ok(entries) = fs::read_dir("/usr/lib/jvm") {
            for entry in entries.flatten() {
                candidates.push(entry.path().join("bin").join("java"));
            }
        }

        candidates
    }

    #[allow(dead_code)]
    pub fn get_installed_java_versions(&self) -> Vec<String> {
        let mut found_versions = Vec::new();
        let mut seen_paths = std::collections::HashSet::new();

        for path in self.java_candidates() {
            if !path.exists() {
                continue;
            }

            if let Ok(path_abs) = std::fs::canonicalize(&path) {
                if !seen_paths.insert(path_abs.clone()) {
                    continue;
                }

                if let Ok(ver) = self.get_java_version(&path_abs) {
                    found_versions.push(format!("Java {} ({})", ver, path_abs.display()));
                }
            }
        }

        found_versions
    }
}
