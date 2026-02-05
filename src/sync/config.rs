/// WebDAV 同步配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebDavConfig {
    pub url: String,
    pub username: String,
    pub password: String,
    pub enabled: bool,
    pub remote_path: String,
}

impl Default for WebDavConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            username: String::new(),
            password: String::new(),
            enabled: false,
            remote_path: "/fish_reader/".to_string(),
        }
    }
}

impl WebDavConfig {
    pub fn load() -> Self {
        let config_path = Self::config_path();
        if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(config) => return config,
                    Err(e) => {
                        eprintln!("Failed to parse webdav.json: {}", e);
                        return Self::default();
                    }
                },
                Err(e) => {
                    eprintln!("Failed to read webdav.json: {}", e);
                    return Self::default();
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let config_path = Self::config_path();

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;

        Ok(())
    }

    fn config_path() -> std::path::PathBuf {
        let mut path = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        path.push(".fish_reader");
        path.push("webdav.json");
        path
    }

    pub fn is_configured(&self) -> bool {
        self.enabled && !self.url.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WebDavConfig::default();
        assert!(!config.enabled);
        assert!(config.url.is_empty());
    }
}
