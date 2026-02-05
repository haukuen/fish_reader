/// 同步元数据，记录版本号和文件状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncMetadata {
    pub version: u64,
    pub timestamp: u64,
    pub file_list: Vec<String>,
}

impl Default for SyncMetadata {
    fn default() -> Self {
        Self {
            version: 0,
            timestamp: 0,
            file_list: Vec::new(),
        }
    }
}

impl SyncMetadata {
    pub fn load() -> Self {
        let meta_path = Self::metadata_path();
        if meta_path.exists() {
            match std::fs::read_to_string(&meta_path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(metadata) => return metadata,
                    Err(e) => {
                        eprintln!("Failed to parse sync_meta.json: {}", e);
                        return Self::default();
                    }
                },
                Err(e) => {
                    eprintln!("Failed to read sync_meta.json: {}", e);
                    return Self::default();
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let meta_path = Self::metadata_path();

        // Ensure parent directory exists
        if let Some(parent) = meta_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&meta_path, content)?;

        Ok(())
    }

    pub fn increment_version(&mut self) {
        self.version += 1;
        self.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    fn metadata_path() -> std::path::PathBuf {
        let mut path = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        path.push(".fish_reader");
        path.push("sync_meta.json");
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_metadata() {
        let meta = SyncMetadata::default();
        assert_eq!(meta.version, 0);
    }

    #[test]
    fn test_increment_version() {
        let mut meta = SyncMetadata::default();
        meta.increment_version();
        assert_eq!(meta.version, 1);
        assert!(meta.timestamp > 0);
    }
}
