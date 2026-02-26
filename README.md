# Fish Reader

一个终端小说阅读器，支持书架管理。

## 安装

### Windows

#### 使用 scoop 安装

添加 endless 仓库
```bash
scoop bucket add endless https://github.com/haukuen/endless
```

安装 fish_reader
```bash
scoop install fish_reader
```

### Other

1.  从 [Releases](https://github.com/haukuen/fish_reader/releases) 页面下载合适版本。
2.  将下载的可执行文件 `fr` 放置在 `PATH` 环境变量所包含的目录中（例如 `/usr/local/bin`）。

## 使用方法

1.  **添加小说**: 将 `.txt` 格式的小说文件复制到 `~/.fish_reader/novels/` 目录下。如果该目录不存在，程序会在首次运行时自动创建。
2.  **运行程序**: 在终端中执行 `fr` 命令启动应用。

## 快捷键

| 快捷键 | 功能 |
| :--- | :--- |
| `q` | 退出程序（输入模式除外） |
| `↑` / `k` | 向上移动 |
| `↓` / `j` | 向下移动 |
| `Enter` | 选择/确认 |
| `Esc` | 回退/取消 |
| `/` | 搜索（阅读页） |
| `t` | 章节列表（阅读页） |
| `b` | 书签列表（阅读页） |
| `m` | 添加书签（阅读页） |
| `s` | 设置 |

## 许可证

本项目使用 [MIT](LICENSE) 许可证。
