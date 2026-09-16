use crate::models::VersionJson;
use anyhow::{anyhow, Result};
use reqwest;
use std::collections::HashSet;
use std::path::PathBuf;
use tokio::fs;
use zip;

#[derive(Clone)]
pub struct LibraryManager {
    versions_dir: PathBuf,
    libraries_dir: PathBuf,
}

impl LibraryManager {
    pub fn new(versions_dir: PathBuf) -> Self {
        let libraries_dir = versions_dir.parent().unwrap().join("libraries");
        Self {
            versions_dir,
            libraries_dir,
        }
    }

    async fn load_version(&self, version: &str) -> Result<VersionJson> {
        let version_file = self
            .versions_dir
            .join(version)
            .join(format!("{}.json", version));

        if !version_file.exists() {
            return Err(anyhow!("Version JSON not found: {:?}", version_file));
        }

        let data = fs::read_to_string(&version_file).await?;
        Ok(serde_json::from_str(&data)?)
    }

    async fn collect_version_chain(&self, start_version: &str) -> Result<Vec<(String, VersionJson)>> {
        let mut chain = Vec::new();
        let mut seen = HashSet::new();
        let mut current = Some(start_version.to_string());

        while let Some(version) = current {
            if !seen.insert(version.clone()) {
                return Err(anyhow!("Circular Minecraft version inheritance detected at {}", version));
            }

            let json = self.load_version(&version).await?;
            current = json.inherits_from.clone();
            chain.push((version, json));
        }

        Ok(chain)
    }

    fn library_artifact(&self, lib: &crate::models::Library, os_name: &str) -> Option<(String, PathBuf)> {
        if !crate::utils::is_library_allowed(lib, os_name) {
            return None;
        }

        if let Some(downloads) = &lib.downloads {
            if let Some(artifact) = &downloads.artifact {
                return Some((artifact.url.clone(), self.libraries_dir.join(&artifact.path)));
            }
        }

        let parts: Vec<&str> = lib.name.split(':').collect();
        if parts.len() < 3 {
            return None;
        }

        let group = parts[0].replace('.', "/");
        let artifact_id = parts[1];
        let version = parts[2];
        let suffix = if parts.len() > 3 {
            format!("-{}", parts[3])
        } else {
            String::new()
        };
        let rel_path = format!(
            "{}/{}/{}/{}-{}{}.jar",
            group, artifact_id, version, artifact_id, version, suffix
        );

        Some((
            format!("https://libraries.minecraft.net/{}", rel_path),
            self.libraries_dir.join(rel_path),
        ))
    }

    async fn download_library(&self, url: &str, path: &PathBuf, name: &str) -> Result<()> {
        if path.exists() {
            return Ok(());
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let response = reqwest::get(url).await.map_err(|e| {
            anyhow!("Failed to download library {} from {}: {}", name, url, e)
        })?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to download library {} from {}: HTTP {}",
                name,
                url,
                response.status()
            ));
        }

        let bytes = response.bytes().await.map_err(|e| {
            anyhow!("Failed to read library {} from {}: {}", name, url, e)
        })?;
        fs::write(path, &bytes).await?;

        Ok(())
    }

    pub async fn check_and_download_libraries(&self, version: &str) -> Result<()> {
        let chain = self.collect_version_chain(version).await?;
        let os_name = crate::utils::get_os_name();
        let mut seen = HashSet::new();

        // Resolve the complete inheritance chain. Parent libraries are needed by
        // child versions too, so only processing the immediate JSON is incorrect.
        for (_version_id, version_json) in chain {
            for lib in version_json.libraries {
                let key = lib.name.clone();
                if !seen.insert(key.clone()) {
                    continue;
                }

                if let Some((url, path)) = self.library_artifact(&lib, os_name) {
                    self.download_library(&url, &path, &key).await?;
                }

                // Native classifiers are separate artifacts and must also be
                // present before extraction/classpath construction.
                if let Some(natives) = &lib.natives {
                    if let Some(classifier) = natives.get(os_name) {
                        if let Some(downloads) = &lib.downloads {
                            if let Some(classifiers) = &downloads.classifiers {
                                if let Some(artifact) = classifiers.get(classifier) {
                                    let path = self.libraries_dir.join(&artifact.path);
                                    self.download_library(&artifact.url, &path, &format!("{} ({})", key, classifier)).await?;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn check_and_extract_natives(&self, natives_version: &str) -> Result<()> {
        let natives_dir = self.versions_dir.join(natives_version).join("natives");

        let natives_ok = natives_dir.exists()
            && std::fs::read_dir(&natives_dir)
                .map(|c| c.count() > 0)
                .unwrap_or(false);

        if natives_ok {
            return Ok(());
        }

        println!(
            "Natives missing for {}, attempting repair...",
            natives_version
        );

        let chain = self.collect_version_chain(natives_version).await?;
        let os_name = crate::utils::get_os_name();

        for (_version_id, v_json) in chain {
            for lib in v_json.libraries {
                let mut native_artifact = None;

                if let Some(natives) = &lib.natives {
                    if let Some(classifier) = natives.get(os_name) {
                        if let Some(downloads) = &lib.downloads {
                            if let Some(classifiers) = &downloads.classifiers {
                                if let Some(artifact) = classifiers.get(classifier) {
                                    native_artifact = Some(artifact.clone());
                                }
                            }
                        }
                    }
                }

                if native_artifact.is_none() {
                    if let Some(downloads) = &lib.downloads {
                        if let Some(classifiers) = &downloads.classifiers {
                            if let Some(artifact) = classifiers.get(&format!("natives-{}", os_name)) {
                                native_artifact = Some(artifact.clone());
                            }
                        }
                    }
                }

                if native_artifact.is_none() {
                    if let Some(downloads) = &lib.downloads {
                        if let Some(artifact) = &downloads.artifact {
                            if artifact.path.contains(&format!("natives-{}", os_name))
                                || lib.name.contains(&format!("natives-{}", os_name))
                            {
                                native_artifact = Some(artifact.clone());
                            }
                        }
                    }
                }

                if let Some(artifact) = native_artifact {
                    let native_zip_path = self
                        .versions_dir
                        .join(natives_version)
                        .join(format!("{}.zip", lib.name.replace(":", "_")));

                    if !native_zip_path.exists() {
                        if let Some(parent) = native_zip_path.parent() {
                            fs::create_dir_all(parent).await?;
                        }
                        let response = reqwest::get(&artifact.url).await.map_err(|e| {
                            anyhow!("Failed to download native library {}: {}", lib.name, e)
                        })?;
                        if !response.status().is_success() {
                            return Err(anyhow!(
                                "Failed to download native library {}: HTTP {}",
                                lib.name,
                                response.status()
                            ));
                        }
                        let bytes = response.bytes().await?;
                        fs::write(&native_zip_path, &bytes).await?;
                    }

                    let nd = natives_dir.clone();
                    let nzp = native_zip_path.clone();
                    let exclude = lib
                        .get_extract()
                        .map(|e| e.exclude.clone())
                        .unwrap_or_default();

                    fs::create_dir_all(&natives_dir).await?;

                    tokio::task::spawn_blocking(move || -> Result<()> {
                        let file = std::fs::File::open(&nzp)?;
                        let mut archive = zip::ZipArchive::new(file)?;
                        for i in 0..archive.len() {
                            let mut file = archive.by_index(i)?;
                            let name = file.name().to_string();
                            let excluded = exclude.iter().any(|ex| name.starts_with(ex));
                            if excluded || name.ends_with('/') {
                                continue;
                            }

                            let filename = std::path::Path::new(&name)
                                .file_name()
                                .and_then(|f| f.to_str())
                                .unwrap_or(&name)
                                .to_string();
                            let outpath = nd.join(filename);

                            let mut outfile = std::fs::File::create(outpath)?;
                            std::io::copy(&mut file, &mut outfile)?;
                        }
                        Ok(())
                    })
                    .await??;
                }
            }
        }

        Ok(())
    }
}