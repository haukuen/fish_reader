use anyhow::Result;

use super::App;

impl App {
    /// 检测孤立的小说记录
    ///
    /// 扫描 library 中所有小说记录，找出 JSON 中存在但文件已被删除的记录。
    pub fn detect_orphaned_novels(&mut self) {
        self.settings.orphaned_novels.clear();

        for novel_info in &self.library.novels {
            if !novel_info.path.exists() {
                self.settings.orphaned_novels.push(novel_info.clone());
            }
        }

        self.settings.selected_orphaned_index = None;
    }

    /// 删除指定索引的小说
    ///
    /// 执行以下操作：
    /// 1. 删除物理文件
    /// 2. 从 novels 列表中移除
    /// 3. 从 library 中移除进度记录
    /// 4. 保存 library 更改
    ///
    /// # Arguments
    ///
    /// * `index` - 要删除的小说在 novels 列表中的索引
    ///
    /// # Errors
    ///
    /// 如果文件删除或保存失败则返回错误。
    pub fn delete_novel(&mut self, index: usize) -> Result<()> {
        if index < self.novels.len() {
            let novel = &self.novels[index];

            if novel.path.exists() {
                std::fs::remove_file(&novel.path)?;
            }

            self.library.novels.retain(|n| n.path != novel.path);

            self.novels.remove(index);

            self.library.save()?;

            if !self.novels.is_empty() {
                let new_index = index.min(self.novels.len() - 1);
                self.settings.selected_delete_novel_index = Some(new_index);
            } else {
                self.settings.selected_delete_novel_index = None;
            }
        }
        Ok(())
    }

    /// 保存当前小说的阅读进度
    ///
    /// 更新并保存当前小说的进度。如果保存失败，会设置错误消息。
    pub fn save_current_progress(&mut self) {
        if let Some(novel) = &self.current_novel {
            self.library
                .update_novel_progress(&novel.path, novel.progress.clone());
            if let Err(e) = self.library.save() {
                self.set_error(format!("Failed to save progress: {}", e));
            }
        }
    }
}
