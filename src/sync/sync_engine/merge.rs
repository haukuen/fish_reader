use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::SyncEngine;

impl SyncEngine {
    /// 合并远程 progress.json 与本地：取较大的 scroll_offset，书签取并集
    pub(super) fn merge_progress(data_dir: &Path, remote_bytes: &[u8]) -> anyhow::Result<()> {
        let progress_path = data_dir.join("progress.json");

        let remote: serde_json::Value = serde_json::from_slice(remote_bytes)?;

        if !progress_path.exists() {
            std::fs::write(&progress_path, remote_bytes)?;
            return Ok(());
        }

        let local: serde_json::Value = match std::fs::read_to_string(&progress_path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
        {
            Some(v) => v,
            None => {
                // 本地损坏或不可读，直接用远程数据覆盖
                std::fs::write(&progress_path, remote_bytes)?;
                return Ok(());
            }
        };

        let merged = Self::merge_library_json(&local, &remote);
        let output = serde_json::to_string_pretty(&merged)?;
        std::fs::write(&progress_path, output)?;

        Ok(())
    }

    /// 按小说合并 Library JSON：取较大 scroll_offset，书签取并集
    pub(super) fn merge_library_json(
        local: &serde_json::Value,
        remote: &serde_json::Value,
    ) -> serde_json::Value {
        let empty_arr = serde_json::Value::Array(vec![]);

        let local_novels = local
            .get("novels")
            .unwrap_or(&empty_arr)
            .as_array()
            .cloned()
            .unwrap_or_default();
        let remote_novels = remote
            .get("novels")
            .unwrap_or(&empty_arr)
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut local_map: HashMap<String, serde_json::Value> = HashMap::new();
        for novel in &local_novels {
            if let Some(title) = novel.get("title").and_then(|t| t.as_str()) {
                local_map.insert(title.to_string(), novel.clone());
            }
        }

        let mut merged_novels: Vec<serde_json::Value> = Vec::new();
        let mut seen_titles: HashSet<String> = HashSet::new();

        for remote_novel in &remote_novels {
            let title = remote_novel
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            seen_titles.insert(title.clone());

            if let Some(local_novel) = local_map.get(&title) {
                merged_novels.push(Self::merge_novel(local_novel, remote_novel));
            } else {
                merged_novels.push(remote_novel.clone());
            }
        }

        for local_novel in &local_novels {
            let title = local_novel
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            if !seen_titles.contains(&title) {
                merged_novels.push(local_novel.clone());
            }
        }

        for novel in &mut merged_novels {
            Self::normalize_novel_json_path(novel);
        }

        serde_json::json!({ "novels": merged_novels })
    }

    pub(super) fn novels_rel_path(path: &str) -> Option<String> {
        let parts: Vec<&str> = path.split(['/', '\\']).filter(|p| !p.is_empty()).collect();
        let novels_idx = parts
            .iter()
            .rposition(|segment| segment.eq_ignore_ascii_case("novels"))?;
        if novels_idx + 1 >= parts.len() {
            return None;
        }
        Some(parts[novels_idx + 1..].join("/"))
    }

    pub(super) fn normalize_novel_json_path(novel: &mut serde_json::Value) {
        let Some(path_str) = novel.get("path").and_then(|p| p.as_str()) else {
            return;
        };
        if let Some(rel) = Self::novels_rel_path(path_str) {
            novel["path"] = serde_json::json!(format!("novels/{}", rel));
        }
    }

    pub(super) fn merge_novel(
        local: &serde_json::Value,
        remote: &serde_json::Value,
    ) -> serde_json::Value {
        let mut merged = remote.clone();
        if let Some(local_path) = local.get("path") {
            merged["path"] = local_path.clone();
        }

        let local_offset = local
            .get("progress")
            .and_then(|p| p.get("scroll_offset"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let remote_offset = remote
            .get("progress")
            .and_then(|p| p.get("scroll_offset"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let max_offset = local_offset.max(remote_offset);

        let empty_arr = serde_json::Value::Array(vec![]);
        let local_bookmarks = local
            .get("progress")
            .and_then(|p| p.get("bookmarks"))
            .unwrap_or(&empty_arr)
            .as_array()
            .cloned()
            .unwrap_or_default();
        let remote_bookmarks = remote
            .get("progress")
            .and_then(|p| p.get("bookmarks"))
            .unwrap_or(&empty_arr)
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut seen_positions: HashSet<u64> = HashSet::new();
        let mut merged_bookmarks: Vec<serde_json::Value> = Vec::new();

        for bm in remote_bookmarks.iter().chain(local_bookmarks.iter()) {
            let pos = bm.get("position").and_then(|p| p.as_u64()).unwrap_or(0);
            if seen_positions.insert(pos) {
                merged_bookmarks.push(bm.clone());
            }
        }
        merged_bookmarks.sort_by_key(|bm| bm.get("position").and_then(|p| p.as_u64()).unwrap_or(0));

        if let Some(progress) = merged.get_mut("progress") {
            progress["scroll_offset"] = serde_json::json!(max_offset);
            progress["bookmarks"] = serde_json::json!(merged_bookmarks);
        }

        merged
    }
}
