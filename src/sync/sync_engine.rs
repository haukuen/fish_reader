use crate::sync::config::WebDavConfig;
use crate::sync::metadata::SyncMetadata;
use crate::sync::webdav_client::{DavResource, WebDavClient};
use std::io::Write;
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;

pub struct SyncEngine {
    client: WebDavClient,
    config: WebDavConfig,
    metadata: SyncMetadata,
}

#[derive(Debug, Clone)]
pub struct Conflict {
    pub local_version: u64,
    pub remote_version: u64,
}

#[derive(Debug, Clone)]
pub struct SyncResult {
    pub downloaded: bool,
    pub uploaded: bool,
    pub new_version: u64,
    pub message: String,
}

impl SyncEngine {
    pub fn new(config: &WebDavConfig) -> anyhow::Result<Self> {
        let client = WebDavClient::new(config)?;
        let metadata = SyncMetadata::load();

        Ok(Self {
            client,
            config: config.clone(),
            metadata,
        })
    }

    pub fn get_local_version(&self) -> u64 {
        self.metadata.version
    }

    pub fn check_conflicts(&self) -> anyhow::Result<Option<Conflict>> {
        let remote_files = self.list_remote_files()?;
        let latest_remote = self.find_latest_version(&remote_files);

        if let Some(remote_ver) = latest_remote {
            if remote_ver > self.metadata.version {
                return Ok(Some(Conflict {
                    local_version: self.metadata.version,
                    remote_version: remote_ver,
                }));
            }
        }

        Ok(None)
    }

    pub fn sync_down(&mut self) -> anyhow::Result<SyncResult> {
        let remote_files = self.list_remote_files()?;
        let latest_file = self.find_latest_file(&remote_files);

        if let Some((filename, remote_version)) = latest_file {
            if remote_version <= self.metadata.version {
                return Ok(SyncResult {
                    downloaded: false,
                    uploaded: false,
                    new_version: self.metadata.version,
                    message: "Already up to date".to_string(),
                });
            }

            let cache_dir = dirs::cache_dir()
                .unwrap_or_else(|| std::env::temp_dir())
                .join("fish_reader");
            std::fs::create_dir_all(&cache_dir)?;

            let temp_file = cache_dir.join(&filename);
            let remote_path = format!("{}/{}", self.config.remote_path, filename);

            self.client.download(&remote_path, &temp_file)?;
            self.unpack_data(&temp_file)?;

            self.metadata.version = remote_version;
            self.metadata.save()?;

            std::fs::remove_file(&temp_file).ok();

            return Ok(SyncResult {
                downloaded: true,
                uploaded: false,
                new_version: remote_version,
                message: format!("Downloaded version {}", remote_version),
            });
        }

        Ok(SyncResult {
            downloaded: false,
            uploaded: false,
            new_version: self.metadata.version,
            message: "No remote data found".to_string(),
        })
    }

    pub fn sync_up(&mut self, force: bool) -> anyhow::Result<SyncResult> {
        if !force && !self.has_local_changes()? {
            return Ok(SyncResult {
                downloaded: false,
                uploaded: false,
                new_version: self.metadata.version,
                message: "No changes to upload".to_string(),
            });
        }

        self.metadata.increment_version();

        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::env::temp_dir())
            .join("fish_reader");
        std::fs::create_dir_all(&cache_dir)?;

        let today = chrono::Local::now().format("%Y%m%d").to_string();
        let filename = format!("{}-{}.fish", today, self.metadata.version);
        let temp_file = cache_dir.join(&filename);

        self.pack_data(&temp_file)?;

        self.cleanup_old_versions()?;

        let remote_path = format!("{}/{}", self.config.remote_path, filename);
        self.client.upload(&temp_file, &remote_path)?;

        self.metadata.save()?;

        std::fs::remove_file(&temp_file).ok();

        Ok(SyncResult {
            downloaded: false,
            uploaded: true,
            new_version: self.metadata.version,
            message: format!("Uploaded version {}", self.metadata.version),
        })
    }

    fn has_local_changes(&self) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn pack_data(&self, output_path: &Path) -> anyhow::Result<()> {
        let file = std::fs::File::create(output_path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        let data_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".fish_reader");

        // Add novels directory
        let novels_dir = data_dir.join("novels");
        if novels_dir.exists() {
            for entry in walkdir::WalkDir::new(&novels_dir) {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("txt") {
                    let relative_path = path.strip_prefix(&data_dir)?;
                    zip.start_file(relative_path.to_string_lossy(), options)?;
                    let contents = std::fs::read(path)?;
                    zip.write_all(&contents)?;
                }
            }
        }

        // Add progress.json
        let progress_file = data_dir.join("progress.json");
        if progress_file.exists() {
            zip.start_file("progress.json", options)?;
            let contents = std::fs::read(&progress_file)?;
            zip.write_all(&contents)?;
        }

        // Add metadata
        zip.start_file("sync_meta.json", options)?;
        let meta_json = serde_json::to_string(&self.metadata)?;
        zip.write_all(meta_json.as_bytes())?;

        zip.finish()?;
        Ok(())
    }

    fn unpack_data(&self, zip_path: &Path) -> anyhow::Result<()> {
        let file = std::fs::File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let data_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".fish_reader");

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = data_dir.join(file.name());

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        Ok(())
    }

    fn list_remote_files(&self) -> anyhow::Result<Vec<DavResource>> {
        self.client.list(&self.config.remote_path)
    }

    fn find_latest_version(&self, files: &[DavResource]) -> Option<u64> {
        files
            .iter()
            .filter_map(|f| self.parse_version(&f.path))
            .max()
    }

    fn find_latest_file(&self, files: &[DavResource]) -> Option<(String, u64)> {
        files
            .iter()
            .filter_map(|f| self.parse_version(&f.path).map(|v| (f.path.clone(), v)))
            .max_by_key(|(_, v)| *v)
    }

    fn parse_version(&self, path: &str) -> Option<u64> {
        let filename = Path::new(path).file_stem()?.to_str()?;
        let parts: Vec<&str> = filename.split('-').collect();
        parts.get(1)?.parse().ok()
    }

    fn cleanup_old_versions(&self) -> anyhow::Result<()> {
        let files = self.list_remote_files()?;
        let mut versioned_files: Vec<(String, u64)> = files
            .iter()
            .filter_map(|f| self.parse_version(&f.path).map(|v| (f.path.clone(), v)))
            .collect();

        versioned_files.sort_by_key(|(_, v)| *v);

        while versioned_files.len() > 10 {
            if let Some((path, _)) = versioned_files.first() {
                self.client.delete(path)?;
                versioned_files.remove(0);
            }
        }

        Ok(())
    }
}
