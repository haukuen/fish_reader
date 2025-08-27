#[derive(Clone, PartialEq)]
pub enum AppState {
    Bookshelf,
    Reading,
    Searching,
    ChapterList,
    Settings,
}

/// 设置界面的子模式
#[derive(Clone, PartialEq, Debug)]
pub enum SettingsMode {
    /// 主菜单：选择删除小说或删除孤立记录
    MainMenu,
    /// 删除小说模式
    DeleteNovel,
    /// 删除孤立记录模式
    DeleteOrphaned,
}
