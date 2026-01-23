/// 应用程序配置常量
pub struct AppConfig {
    /// 应用目录名称
    pub dir_name: &'static str,
    /// 支持的小说文件扩展名
    pub supported_extensions: &'static [&'static str],
    /// 进度文件名
    pub progress_filename: &'static str,
    /// 备份文件前缀
    pub backup_prefix: &'static str,
    /// 备份文件时间戳格式（10分钟间隔）
    pub backup_timestamp_interval: u64,
    /// 备份保留天数
    pub backup_retention_days: u64,
    /// 设置菜单项数量
    pub settings_menu_count: usize,
}

impl AppConfig {
    /// 创建默认配置
    pub const fn default() -> Self {
        Self {
            dir_name: ".fish_reader",
            supported_extensions: &["txt"],
            progress_filename: "progress.json",
            backup_prefix: "progress.json.backup",
            backup_timestamp_interval: 600, // 10分钟
            backup_retention_days: 3,
            settings_menu_count: 2,
        }
    }
}

/// 全局配置实例
pub const CONFIG: AppConfig = AppConfig::default();
