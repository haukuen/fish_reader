use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Novel {
    pub title: String,
    pub path: PathBuf,
    pub content: String,
    pub progress: ReadingProgress,
}

impl Novel {
    pub fn new(path: PathBuf) -> Self {
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("未知标题")
            .to_string();

        Novel {
            title,
            path: path.clone(),
            content: String::new(),
            progress: ReadingProgress::default(),
        }
    }

    pub fn load_content(&mut self) -> std::io::Result<()> {
        self.content = std::fs::read_to_string(&self.path)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct ReadingProgress {
    pub line: usize,
    pub scroll_offset: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Library {
    pub novels: Vec<NovelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelInfo {
    pub title: String,
    pub path: PathBuf,
    pub progress: ReadingProgress,
}

impl Library {
    pub fn new() -> Self {
        Library { novels: Vec::new() }
    }

    pub fn load() -> Self {
        let progress_path = Self::get_progress_path();
        if progress_path.exists() {
            match std::fs::read_to_string(&progress_path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(library) => return library,
                    Err(_) => return Self::new(),
                },
                Err(_) => return Self::new(),
            }
        }
        Self::new()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let progress_path = Self::get_progress_path();
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(progress_path, content)?;
        Ok(())
    }

    pub fn get_progress_path() -> PathBuf {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".fish_reader");

        if !path.exists() {
            let _ = std::fs::create_dir_all(&path);
        }

        path.push("progress.json");
        path
    }

    pub fn update_novel_progress(&mut self, novel_path: &Path, progress: ReadingProgress) {
        if let Some(novel) = self.novels.iter_mut().find(|n| n.path == novel_path) {
            novel.progress = progress;
        } else {
            let title = novel_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("未知标题")
                .to_string();

            self.novels.push(NovelInfo {
                title,
                path: novel_path.to_path_buf(),
                progress,
            });
        }
    }

    pub fn get_novel_progress(&self, novel_path: &Path) -> ReadingProgress {
        self.novels
            .iter()
            .find(|n| n.path == novel_path)
            .map(|n| n.progress)
            .unwrap_or_default()
    }
}
