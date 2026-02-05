use crate::sync::config::WebDavConfig;
use reqwest::blocking::Client;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct DavResource {
    pub path: String,
    pub size: u64,
    pub modified_time: Option<u64>,
    pub is_dir: bool,
}

pub struct WebDavClient {
    client: Client,
    base_url: String,
    username: String,
    password: String,
}

impl WebDavClient {
    pub fn new(config: &WebDavConfig) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: config.url.trim_end_matches('/').to_string(),
            username: config.username.clone(),
            password: config.password.clone(),
        })
    }

    pub fn list(&self, path: &str) -> anyhow::Result<Vec<DavResource>> {
        let url = format!("{}{}", self.base_url, path);

        let request = self
            .client
            .request(reqwest::Method::from_bytes(b"PROPFIND")?, &url)
            .header("Depth", "1");

        let request = if !self.username.is_empty() {
            request.basic_auth(&self.username, Some(&self.password))
        } else {
            request
        };

        let response = request.send()?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("PROPFIND failed: {}", response.status()));
        }

        let body = response.text()?;
        self.parse_propfind(&body)
    }

    pub fn download(&self, remote_path: &str, local_path: &Path) -> anyhow::Result<()> {
        let url = format!("{}{}", self.base_url, remote_path);

        let request = self.client.get(&url);
        let request = if !self.username.is_empty() {
            request.basic_auth(&self.username, Some(&self.password))
        } else {
            request
        };

        let response = request.send()?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Download failed: {}", response.status()));
        }

        let bytes = response.bytes()?;
        std::fs::write(local_path, bytes)?;

        Ok(())
    }

    pub fn upload(&self, local_path: &Path, remote_path: &str) -> anyhow::Result<()> {
        let url = format!("{}{}", self.base_url, remote_path);
        let data = std::fs::read(local_path)?;

        let request = self.client.put(&url).body(data);
        let request = if !self.username.is_empty() {
            request.basic_auth(&self.username, Some(&self.password))
        } else {
            request
        };

        let response = request.send()?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Upload failed: {}", response.status()));
        }

        Ok(())
    }

    pub fn delete(&self, remote_path: &str) -> anyhow::Result<()> {
        let url = format!("{}{}", self.base_url, remote_path);

        let request = self.client.delete(&url);
        let request = if !self.username.is_empty() {
            request.basic_auth(&self.username, Some(&self.password))
        } else {
            request
        };

        let response = request.send()?;

        if !response.status().is_success() && response.status() != reqwest::StatusCode::NOT_FOUND {
            return Err(anyhow::anyhow!("Delete failed: {}", response.status()));
        }

        Ok(())
    }

    fn parse_propfind(&self, xml: &str) -> anyhow::Result<Vec<DavResource>> {
        let mut resources = Vec::new();

        for line in xml.lines() {
            if line.contains("<d:href>") || line.contains("<D:href>") {
                if let Some(start) = line.find(">") {
                    if let Some(end) = line.rfind("<") {
                        let path = line[start + 1..end].to_string();
                        if !path.ends_with('/') {
                            resources.push(DavResource {
                                path,
                                size: 0,
                                modified_time: None,
                                is_dir: false,
                            });
                        }
                    }
                }
            }
        }

        Ok(resources)
    }
}
