use anyhow::{anyhow, Result};
use flate2::read::GzDecoder;
use futures::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tar::Archive;

#[derive(Deserialize, Debug)]
struct AdoptiumRelease {
    binaries: Vec<AdoptiumBinary>,
}

#[derive(Deserialize, Debug)]
struct AdoptiumBinary {
    package: AdoptiumPackage,
}

#[derive(Deserialize, Debug)]
struct AdoptiumPackage {
    link: String,
}

#[derive(Clone)]
pub struct JavaManager {
    runtimes_dir: PathBuf,
}

impl JavaManager {
    pub fn new() -> Self {
        let runtimes_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".minecraft")
            .join("runtime");
        Self { runtimes_dir }
    }

    pub async fn find_or_install_java<F>(&self, required_version: u32, on_progress: F) -> Result<PathBuf>
    where
        F: Fn(f64, String) + Send + Sync + 'static,
    {
        let target_dir = self.runtimes_dir.join(format!("java-{}", required_version));
        let java_bin = target_dir.join("bin").join("java");

        if java_bin.exists() && Self::java_matches_version(&java_bin, required_version) {
            on_progress(1.0, format!("Using Java {}", required_version));
            return Ok(java_bin);
        }

        self.download_and_install_java(required_version, on_progress).await
    }

    async fn download_and_install_java<F>(&self, version: u32, on_progress: F) -> Result<PathBuf>
    where
        F: Fn(f64, String) + Send + Sync + 'static,
    {
        let target_dir = self.runtimes_dir.join(format!("java-{}", version));
        let java_bin = target_dir.join("bin").join("java");

        on_progress(0.0, format!("Finding Java {}...", version));
        fs::create_dir_all(&self.runtimes_dir)?;

        let url = format!(
            "https://api.adoptium.net/v3/assets/feature_releases/{}/ga?architecture=x64&heap_size=normal&image_type=jdk&jvm_impl=hotspot&os=linux",
            version
        );

        let client = Client::new();
        let response = client.get(&url).send().await?.error_for_status()?;
        let releases: Vec<AdoptiumRelease> = response.json().await?;
        let download_url = releases
            .first()
            .and_then(|release| release.binaries.first())
            .map(|binary| binary.package.link.clone())
            .ok_or_else(|| anyhow!("No Java {} runtime found", version))?;

        on_progress(0.1, format!("Downloading Java {}...", version));
        let response = client.get(&download_url).send().await?.error_for_status()?;
        let total_size = response.content_length().unwrap_or(0);
        let mut stream = response.bytes_stream();
        let mut downloaded = 0u64;
        let mut archive_bytes = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded += chunk.len() as u64;
            archive_bytes.extend_from_slice(&chunk);
            if total_size > 0 {
                let progress = 0.1 + 0.6 * (downloaded as f64 / total_size as f64);
                on_progress(progress, format!("Downloading Java {}... ({:.1} MB)", version, downloaded as f64 / 1024.0 / 1024.0));
            }
        }

        on_progress(0.7, "Extracting Java Runtime...".to_string());
        let temp_dir = self.runtimes_dir.join(format!(".tmp-java-{}", version));
        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir)?;
        }
        fs::create_dir_all(&temp_dir)?;

        let result = (|| -> Result<PathBuf> {
            if download_url.ends_with(".zip") {
                let reader = Cursor::new(archive_bytes);
                let mut archive = zip::ZipArchive::new(reader)?;
                archive.extract(&temp_dir)?;
            } else {
                let tar = GzDecoder::new(Cursor::new(archive_bytes));
                let mut archive = Archive::new(tar);
                archive.unpack(&temp_dir)?;
            }

            let extracted_root = fs::read_dir(&temp_dir)?
                .flatten()
                .map(|entry| entry.path())
                .find(|path| path.is_dir())
                .ok_or_else(|| anyhow!("Java archive extracted no runtime directory"))?;

            if target_dir.exists() {
                fs::remove_dir_all(&target_dir)?;
            }
            fs::rename(&extracted_root, &target_dir)?;

            if !java_bin.exists() {
                return Err(anyhow!("Java {} binary not found after extraction", version));
            }

            #[cfg(target_family = "unix")]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut permissions = fs::metadata(&java_bin)?.permissions();
                permissions.set_mode(0o755);
                fs::set_permissions(&java_bin, permissions)?;
            }

            if !Self::java_matches_version(&java_bin, version) {
                return Err(anyhow!("Installed Java runtime reports the wrong version"));
            }

            Ok(java_bin.clone())
        })();

        let _ = fs::remove_dir_all(&temp_dir);
        result
    }

    fn java_matches_version(path: &Path, required_version: u32) -> bool {
        Self::get_java_version(path)
            .map(|version| version == required_version)
            .unwrap_or(false)
    }

    fn get_java_version(path: &Path) -> Result<u32> {
        let output = StdCommand::new(path).arg("-version").output()?;
        let version_output = format!("{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));

        for line in version_output.lines() {
            if !line.contains("version") {
                continue;
            }
            let parts: Vec<&str> = line.split('"').collect();
            if parts.len() < 2 {
                continue;
            }
            let version = parts[1];
            if let Some(minor) = version.strip_prefix("1.").and_then(|v| v.split('.').next()) {
                return minor.parse::<u32>().map_err(|_| anyhow!("Invalid Java version"));
            }
            if let Some(major) = version.split('.').next() {
                return major.parse::<u32>().map_err(|_| anyhow!("Invalid Java version"));
            }
        }
        Err(anyhow!("Could not determine Java version"))
    }

    pub fn get_installed_java_versions(&self) -> Vec<String> {
        let mut found = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.runtimes_dir) {
            for entry in entries.flatten() {
                let path = entry.path().join("bin").join("java");
                if path.exists() {
                    if let Ok(version) = Self::get_java_version(&path) {
                        found.push(format!("Java {} ({})", version, path.display()));
                    }
                }
            }
        }
        found
    }

    pub fn runtimes_dir(&self) -> &Path {
        &self.runtimes_dir
    }

    pub fn find_java(&self) -> Result<PathBuf> {
        let entries = fs::read_dir(&self.runtimes_dir)
            .map_err(|_| anyhow!("No managed Java runtimes are installed"))?;
        for entry in entries.flatten() {
            let path = entry.path().join("bin").join("java");
            if path.exists() && Self::get_java_version(&path).is_ok() {
                return Ok(path);
            }
        }
        Err(anyhow!("No managed Java runtime is installed"))
    }
}
