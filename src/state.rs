/// 应用程序主状态
///
/// 表示用户当前所处的界面或操作模式。
#[derive(Clone, PartialEq)]
pub enum AppState {
    Bookshelf,
    Reading,
    Searching,
    ChapterList,
    Settings,
    BookmarkList,
    BookmarkAdd,
}

/// 设置界面的子模式
///
/// 表示设置页面内的不同操作状态。
#[derive(Clone, PartialEq, Debug, Default)]
pub enum SettingsMode {
    /// 主菜单：选择删除小说或删除孤立记录
    #[default]
    MainMenu,
    /// 删除小说模式
    DeleteNovel,
    /// 删除孤立记录模式
    DeleteOrphaned,
    /// WebDAV配置模式
    WebDavConfig,
}
