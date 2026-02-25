use super::App;

impl App {
    /// 在当前小说内容中搜索关键词
    ///
    /// 执行不区分大小写的搜索，更新搜索结果列表。
    ///
    /// # Note
    ///
    /// 搜索输入为空时会清空结果列表。
    pub fn perform_search(&mut self) {
        if let Some(novel) = &self.current_novel {
            if !self.search.input.is_empty() {
                self.search.results.clear();

                let search_term = self.search.input.to_lowercase();

                for (line_num, line) in novel.lines().iter().enumerate() {
                    if line.to_lowercase().contains(&search_term) {
                        self.search.results.push((line_num, line.clone()));
                    }
                }

                if !self.search.results.is_empty() {
                    let should_reset = match self.search.selected_index {
                        None => true,
                        Some(idx) => idx >= self.search.results.len(),
                    };
                    if should_reset {
                        self.search.selected_index = Some(0);
                    }
                } else {
                    self.search.selected_index = None;
                }
            } else {
                self.search.results.clear();
                self.search.selected_index = None;
            }
        }
    }

    /// 根据当前阅读位置查找对应的章节索引
    ///
    /// # Returns
    ///
    /// 返回最接近当前阅读位置的章节索引。如果没有章节或当前未打开小说，返回 `None`。
    pub fn find_current_chapter_index(&self) -> Option<usize> {
        self.current_novel.as_ref().and_then(|novel| {
            if novel.chapters.is_empty() {
                return None;
            }
            Some(Self::find_chapter_index(
                &novel.chapters,
                novel.progress.scroll_offset,
            ))
        })
    }

    /// 查找指定行所在的章节索引
    ///
    /// # Arguments
    ///
    /// * `chapters` - 章节列表
    /// * `current_line` - 当前行号
    ///
    /// # Returns
    ///
    /// 最接近当前行的章节索引（即 `start_line` 小于等于 `current_line` 的最大索引）。
    pub fn find_chapter_index(
        chapters: &[crate::model::novel::Chapter],
        current_line: usize,
    ) -> usize {
        let mut current_idx = 0;
        for (index, chapter) in chapters.iter().enumerate() {
            if chapter.start_line <= current_line {
                current_idx = index;
            } else {
                break;
            }
        }
        current_idx
    }
}
