use crate::model::library::Library;
use crate::sync::sync_engine::{SyncEngine, SyncMessage};
use crate::ui::sync_status::SyncStatus;

use super::App;

impl App {
    /// 手动上传同步（后台线程执行）
    pub fn trigger_sync(&mut self) {
        if self.sync_status.is_busy() {
            return;
        }
        if !self.webdav_config.is_configured() {
            self.set_error("请先配置 WebDAV");
            return;
        }

        let config = self.webdav_config.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.sync_rx = Some(rx);
        self.sync_status = SyncStatus::InProgress("准备上传...".into());

        std::thread::spawn(move || match SyncEngine::new(&config) {
            Ok(engine) => engine.sync_up(&tx),
            Err(e) => {
                tx.send(SyncMessage::Failed(e.to_string())).ok();
            }
        });
    }

    /// 手动下载同步（后台线程执行）
    pub fn trigger_download(&mut self) {
        if self.sync_status.is_busy() {
            return;
        }
        if !self.webdav_config.is_configured() {
            self.set_error("请先配置 WebDAV");
            return;
        }

        let config = self.webdav_config.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.sync_rx = Some(rx);
        self.sync_status = SyncStatus::InProgress("准备下载...".into());

        std::thread::spawn(move || match SyncEngine::new(&config) {
            Ok(engine) => engine.sync_down(&tx),
            Err(e) => {
                tx.send(SyncMessage::Failed(e.to_string())).ok();
            }
        });
    }

    /// 轮询同步状态（主循环中调用）
    pub fn poll_sync_status(&mut self) {
        let Some(rx) = &self.sync_rx else { return };

        while let Ok(msg) = rx.try_recv() {
            match msg {
                SyncMessage::Progress(text) => {
                    self.sync_status = SyncStatus::InProgress(text);
                }
                SyncMessage::UploadComplete => {
                    self.sync_status = SyncStatus::Success("上传完成".into());
                    self.sync_rx = None;
                    return;
                }
                SyncMessage::DownloadComplete => {
                    if let Ok(novels) = Self::load_novels_from_dir(&Self::get_novels_dir()) {
                        self.novels = novels;
                    }
                    self.library = Library::load();
                    self.sync_status = SyncStatus::Success("下载完成".into());
                    self.sync_rx = None;
                    return;
                }
                SyncMessage::Failed(err) => {
                    self.sync_status = SyncStatus::Error(err);
                    self.sync_rx = None;
                    return;
                }
            }
        }
    }
}
