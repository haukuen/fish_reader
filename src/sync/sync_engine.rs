use crate::sync::config::WebDavConfig;
use crate::sync::webdav_client::WebDavClient;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use zip::write::SimpleFileOptions;

/// 同步进度消息
pub enum SyncMessage {
    /// 进度更新（显示在状态栏）
    Progress(String),
    /// 上传完成
    UploadComplete,
    /// 下载完成（需要重新加载数据）
    DownloadComplete,
    /// 操作失败
    Failed(String),
}

pub struct SyncEngine {
    client: WebDavClient,
    config: WebDavConfig,
}

impl SyncEngine {
    pub fn new(config: &WebDavConfig) -> anyhow::Result<Self> {
        let client = WebDavClient::new(config)?;
        Ok(Self {
            client,
            config: config.clone(),
        })
    }

    /// 上传同步（后台线程调用）
    pub fn sync_up(&self, tx: &Sender<SyncMessage>) {
        if let Err(e) = self.do_sync_up(tx) {
            tx.send(SyncMessage::Failed(e.to_string())).ok();
        }
    }

    /// 下载同步（后台线程调用）
    pub fn sync_down(&self, tx: &Sender<SyncMessage>) {
        if let Err(e) = self.do_sync_down(tx) {
            tx.send(SyncMessage::Failed(e.to_string())).ok();
        }
    }

    fn do_sync_up(&self, tx: &Sender<SyncMessage>) -> anyhow::Result<()> {
        let cache_dir = std::env::temp_dir().join("fish_reader");
        std::fs::create_dir_all(&cache_dir)?;

        let filename = format!(
            "{}.fish",
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        );
        let temp_file = cache_dir.join(&filename);

        tx.send(SyncMessage::Progress("打包数据中...".into())).ok();
        self.pack_data(&temp_file)?;

        tx.send(SyncMessage::Progress(format!("上传 {}...", filename)))
            .ok();
        let remote_path = self.remote_file_path(&filename);
        match self.client.upload(&temp_file, &remote_path) {
            Ok(()) => {
                std::fs::remove_file(&temp_file).ok();
                tx.send(SyncMessage::Progress("清理旧版本...".into())).ok();
                self.cleanup_old_versions().ok();
                tx.send(SyncMessage::UploadComplete).ok();
                Ok(())
            }
            Err(e) => {
                std::fs::remove_file(&temp_file).ok();
                Err(e)
            }
        }
    }

    fn do_sync_down(&self, tx: &Sender<SyncMessage>) -> anyhow::Result<()> {
        tx.send(SyncMessage::Progress("查询远程文件...".into())).ok();
        let remote_files = self.list_remote_files()?;
        let filename = self
            .find_latest_file(&remote_files)
            .ok_or_else(|| anyhow::anyhow!("远程没有同步数据"))?;

        let cache_dir = std::env::temp_dir().join("fish_reader");
        std::fs::create_dir_all(&cache_dir)?;

        let temp_file = cache_dir.join(&filename);
        let remote_path = self.remote_file_path(&filename);

        tx.send(SyncMessage::Progress(format!("下载 {}...", filename)))
            .ok();
        self.client.download(&remote_path, &temp_file)?;

        tx.send(SyncMessage::Progress("解压数据中...".into())).ok();
        self.unpack_data(&temp_file)?;

        std::fs::remove_file(&temp_file).ok();
        tx.send(SyncMessage::DownloadComplete).ok();
        Ok(())
    }

    fn pack_data(&self, output_path: &Path) -> anyhow::Result<()> {
        let file = std::fs::File::create(output_path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        let data_dir = home::home_dir()
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

        zip.finish()?;
        Ok(())
    }

    fn unpack_data(&self, zip_path: &Path) -> anyhow::Result<()> {
        let file = std::fs::File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let data_dir = home::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".fish_reader");

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;

            // 路径穿越保护：使用 enclosed_name() 防止 Zip Slip 攻击
            let Some(enclosed) = file.enclosed_name() else {
                continue;
            };
            let outpath = data_dir.join(enclosed);

            if file.is_dir() {
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

    fn list_remote_files(&self) -> anyhow::Result<Vec<crate::sync::webdav_client::DavResource>> {
        self.client.list(&self.config.remote_path)
    }

    /// 按文件名排序找到最新的 .fish 文件（YYYYMMDD-HHMMSS 格式天然支持字典序排序）
    fn find_latest_file(
        &self,
        files: &[crate::sync::webdav_client::DavResource],
    ) -> Option<String> {
        files
            .iter()
            .filter_map(|f| {
                let filename = Path::new(&f.path).file_name()?.to_str()?.to_string();
                if filename.ends_with(".fish") {
                    Some(filename)
                } else {
                    None
                }
            })
            .max()
    }

    /// 构造远程文件路径：remote_path + filename
    fn remote_file_path(&self, filename: &str) -> String {
        format!(
            "{}/{}",
            self.config.remote_path.trim_end_matches('/'),
            filename
        )
    }

    fn cleanup_old_versions(&self) -> anyhow::Result<()> {
        let files = self.list_remote_files()?;
        let mut fish_files: Vec<String> = files
            .iter()
            .filter_map(|f| {
                let filename = Path::new(&f.path).file_name()?.to_str()?.to_string();
                if filename.ends_with(".fish") {
                    Some(filename)
                } else {
                    None
                }
            })
            .collect();

        fish_files.sort();

        while fish_files.len() > 10 {
            if let Some(filename) = fish_files.first() {
                let remote_path = self.remote_file_path(filename);
                self.client.delete(&remote_path)?;
                fish_files.remove(0);
            }
        }

        Ok(())
    }
}
