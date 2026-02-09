use crate::sync::config::WebDavConfig;
use reqwest::blocking::Client;

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

    pub fn mkcol(&self, remote_path: &str) -> anyhow::Result<()> {
        let url = format!("{}{}", self.base_url, remote_path);

        let request = self
            .client
            .request(reqwest::Method::from_bytes(b"MKCOL")?, &url);
        let request = if !self.username.is_empty() {
            request.basic_auth(&self.username, Some(&self.password))
        } else {
            request
        };

        let response = request.send()?;

        // 405 Method Not Allowed means directory already exists
        if !response.status().is_success()
            && response.status() != reqwest::StatusCode::METHOD_NOT_ALLOWED
        {
            return Err(anyhow::anyhow!("MKCOL failed: {}", response.status()));
        }

        Ok(())
    }

    pub fn upload_bytes(&self, data: &[u8], remote_path: &str) -> anyhow::Result<()> {
        let url = format!("{}{}", self.base_url, remote_path);

        let request = self.client.put(&url).body(data.to_vec());
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

    pub fn download_bytes(&self, remote_path: &str) -> anyhow::Result<Vec<u8>> {
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

        Ok(response.bytes()?.to_vec())
    }

    pub fn test_connection(&self, remote_path: &str) -> anyhow::Result<()> {
        let url = format!("{}{}", self.base_url, remote_path);

        let request = self
            .client
            .request(reqwest::Method::from_bytes(b"PROPFIND")?, &url)
            .header("Depth", "0");
        let request = if !self.username.is_empty() {
            request.basic_auth(&self.username, Some(&self.password))
        } else {
            request
        };

        let response = request.send()?;

        if response.status().is_success() || response.status().as_u16() == 207 {
            Ok(())
        } else {
            Err(anyhow::anyhow!("HTTP {}", response.status()))
        }
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
}
