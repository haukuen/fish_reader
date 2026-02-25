use super::App;

impl App {
    /// 在当前小说的阅读位置添加书签
    ///
    /// # Arguments
    ///
    /// * `name` - 书签名称
    pub fn add_bookmark(&mut self, name: String) {
        if let Some(novel) = &mut self.current_novel {
            let position = novel.progress.scroll_offset;
            novel.progress.add_bookmark(name, position);
            self.save_current_progress();
        }
    }

    /// 删除当前小说的指定书签
    ///
    /// # Arguments
    ///
    /// * `index` - 要删除的书签索引
    ///
    /// # Returns
    ///
    /// 如果删除成功返回 `Some(())`，如果索引无效或当前无小说则返回 `None`。
    pub fn remove_bookmark(&mut self, index: usize) -> Option<()> {
        if let Some(novel) = &mut self.current_novel
            && novel.progress.remove_bookmark(index).is_some()
        {
            self.save_current_progress();
            Some(())
        } else {
            None
        }
    }

    /// 跳转到指定书签位置
    ///
    /// # Arguments
    ///
    /// * `index` - 要跳转的书签索引
    ///
    /// # Returns
    ///
    /// 如果跳转成功返回 `Some(())`，如果索引无效或当前无小说则返回 `None`。
    pub fn jump_to_bookmark(&mut self, index: usize) -> Option<()> {
        if let Some(novel) = &mut self.current_novel
            && let Some(bookmark) = novel.progress.bookmarks.get(index)
        {
            novel.progress.scroll_offset = bookmark.position;
            self.save_current_progress();
            Some(())
        } else {
            None
        }
    }

    /// 获取当前小说的书签列表
    ///
    /// # Returns
    ///
    /// 如果当前有打开的小说则返回其书签列表的引用，否则返回 `None`。
    pub fn get_current_bookmarks(&self) -> Option<&Vec<crate::model::novel::Bookmark>> {
        self.current_novel
            .as_ref()
            .map(|novel| &novel.progress.bookmarks)
    }

    /// 清空书签输入框内容
    pub fn clear_bookmark_inputs(&mut self) {
        self.bookmark.clear_input();
    }
}
