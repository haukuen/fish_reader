use crate::sync::config::WebDavConfig;
use reqwest::blocking::Client;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct DavResource {
    pub path: String,
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
            .timeout(std::time::Duration::from_secs(10))
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

        // 支持常见 WebDAV 命名空间前缀，可处理多行 XML
        for tag in ["<d:href>", "<D:href>", "<href>"] {
            let close_tag = tag.replace('<', "</");
            let mut search_from = 0;

            while let Some(start) = xml[search_from..].find(tag) {
                let content_start = search_from + start + tag.len();
                if let Some(end) = xml[content_start..].find(&close_tag) {
                    let path = xml[content_start..content_start + end]
                        .trim()
                        .to_string();
                    if !path.is_empty() && !path.ends_with('/') {
                        resources.push(DavResource {
                            path,
                        });
                    }
                    search_from = content_start + end + close_tag.len();
                } else {
                    break;
                }
            }
        }

        Ok(resources)
    }
}
