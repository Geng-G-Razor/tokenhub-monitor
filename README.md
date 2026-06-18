# TokenHub Monitor

跨平台桌面托盘应用，监控 [tokenhub.cash](https://tokenhub.cash) 的 API 套餐用量。点击托盘图标即可查看实时用量、配额进度、模型支持等数据。

支持 **macOS** 和 **Windows 11**。

## 功能

- **托盘常驻**：托盘图标旁显示今日用量数值，左键点击弹出面板
  - macOS：面板从菜单栏向下弹出
  - Windows：面板从任务栏向上弹出
- **API Key 登录**：粘贴 Bearer Token 即可使用，Token 存储在系统凭据管理器中（macOS Keychain / Windows Credential Manager）
- **实时用量**：总配额消耗、本周用量、RPM 限制
- **进度条**：总配额和每周限额进度可视化
- **模型列表**：显示套餐支持的模型列表
- **定时刷新**：支持 1 / 5 / 10 / 30 分钟间隔，窗口获焦时自动刷新
- **智能隐藏**：鼠标离开托盘和面板区域后自动隐藏（macOS 3 秒 / 其他 500ms）
- **登录态管理**：登出清除凭据，下次启动自动恢复

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面框架 | Tauri 2 |
| 前端 | TypeScript + Vite，原生 HTML/CSS |
| 后端 | Rust（reqwest + keyring + thiserror） |
| 凭据存储 | macOS Keychain / Windows Credential Manager（keyring crate） |

## 项目结构

```
tokenhub-monitor/
├── package.json              # 前端依赖及脚本
├── vite.config.ts            # Vite 构建配置
├── index.html                # 入口 HTML（加载 / 登录 / 仪表盘三视图）
├── src/                      # 前端
│   ├── main.ts               # invoke Tauri commands、定时刷新、面板交互
│   └── style.css             # 暗色主题样式
└── src-tauri/
    ├── Cargo.toml             # Rust 依赖（按平台条件编译）
    ├── tauri.conf.json        # 窗口、托盘、打包配置
    ├── icons/                 # 应用图标（多平台多尺寸）
    └── src/
        ├── main.rs            # 入口，调用 lib::run()
        ├── lib.rs             # Tauri Builder + 托盘注册 + 窗口智能隐藏
        ├── commands.rs        # Tauri commands: save/clear/has_master_key, fetch_package
        ├── api.rs             # reqwest HTTP 客户端（平台特定 User-Agent）
        └── auth.rs            # 跨平台凭据存储
```

## 核心模块说明

### 前端 (`src/main.ts`)

- **三视图切换**：loading → login → dash，通过 `show()` 函数控制显隐
- **视图感知自动隐藏**：`show()` 在切换视图时调用 `set_auto_hide` — 登录视图禁用自动隐藏（防止输入时窗口消失），Dashboard 视图启用
- **数据渲染**：`render(PackageData)` 将后端返回的数据映射到 DOM；用量数字使用完整格式（`1,234,567`），配额总量使用紧凑格式（`1.2M`）
- **窗口高度自适应**：每次渲染后调用 `fitHeight()` 测量内容高度并通知后端 `fit_height`，消除底部空白
- **托盘标题更新**：每次刷新后通过 `invoke("set_tray_title")` 更新托盘标题为用量数值
- **定时器**：`setInterval` 轮询，用户可切换刷新间隔
- **焦点刷新**：`onFocusChanged` 在面板获焦时立即刷新
- **鼠标进出监听**：通知后端鼠标状态，配合智能隐藏逻辑

### 后端 Rust

#### `lib.rs` — 应用入口与托盘

- 构建系统托盘图标，**左键点击**弹出 / 隐藏面板窗口
- 面板锚定在点击位置：
  - macOS：面板在点击位置下方弹出（菜单栏在顶部）
  - Windows：面板在点击位置上方弹出（任务栏在底部），并记录锚点 Y 坐标
- 窗口宽度固定 340，高度自适应内容
- 窗口智能隐藏机制（多信号冗余触发，解决 Windows 焦点事件不可靠问题）：
  - `ALLOW_AUTO_HIDE` / `PENDING_HIDE` / `MOUSE_ON_TRAY` / `MOUSE_IN_WINDOW` 四个 `AtomicBool` 控制
  - 触发信号包括：Tray Leave、前端 mouseleave、前端 `window.blur`、WindowEvent::Focused(false)
  - 4 个信号中任一触发即启动延迟隐藏计时器
  - 隐藏条件：延迟期满时 `PENDING_HIDE && ALLOW_AUTO_HIDE && !MOUSE_ON_TRAY && !MOUSE_IN_WINDOW`

#### `lib.rs` — Tauri Commands

| 命令 | 说明 |
|------|------|
| `set_auto_hide(enabled)` | 控制是否允许失焦自动隐藏（登录时禁用） |
| `set_mouse_in_window(in_window)` | 鼠标进出面板窗口通知 |
| `start_hide_timer_cmd()` | 前端主动触发隐藏计时器（用于 `window.blur`） |
| `fit_height(height)` | 按内容高度调整窗口，Windows 底部重新锚定 |
| `set_tray_title(title)` | 更新托盘图标旁标题 |
| `quit_app()` | 退出应用 |

#### `commands.rs` — 业务 Commands

| 命令 | 签名 | 说明 |
|------|------|------|
| `save_master_key` | `(key) → ()` | 保存 API Key 到系统凭据管理器 |
| `clear_master_key` | `() → ()` | 清除凭据 |
| `has_master_key` | `() → bool` | 检查是否有已存储的 Key |
| `fetch_package` | `() → PackageData` | 拉取套餐用量数据 |

#### `api.rs` — HTTP 客户端

- 基础地址：`https://api.tokenhub.cash`
- 请求头包含 `Authorization: Bearer {key}` 和平台特定 User-Agent
- 20 秒超时，rustls TLS
- 统一错误处理

#### `auth.rs` — 凭据存储

- 跨平台：macOS Keychain 和 Windows Credential Manager
- 存储 `master_key`（用户 `tokenhub-api-key`），Service 名 `cash.tokenhub.monitor`

### 窗口与 UI 配置 (`tauri.conf.json`)

- 340 宽无边框透明窗口，常驻顶层，不在任务栏显示
- 打包格式：
  - macOS：`.app` + `.dmg`
  - Windows：`.msi`（WiX）+ `.exe`（NSIS）

## 安装

### macOS

1. 从 [Releases](https://github.com/Geng-G-Razor/tokenhub-monitor/releases) 下载 `.dmg`
2. 打开 DMG，将 App 拖入 Applications 文件夹
3. **首次打开提示"未识别的开发者"时**，需要移除签名隔离属性：

   ```bash
   sudo xattr -rd com.apple.quarantine /Applications/TokenHub\ Monitor.app
   ```

   > 本项目为**无签名打包**（unsigned build），因此 macOS Gatekeeper 会阻止直接打开。移除 quarantine 属性即可正常运行。
   >
   > 如需消除此提示，可自行使用 Apple Developer 证书签名：
   > ```bash
   > codesign --force --sign "Developer ID Application: YOUR_NAME" /Applications/TokenHub\ Monitor.app
   > ```

### Windows

1. 从 [Releases](https://github.com/Geng-G-Razor/tokenhub-monitor/releases) 下载 `.msi` 或 `.exe`
2. 运行安装程序
3. Windows SmartScreen 可能提示"Windows 已保护你的电脑"，点击"仍要运行"即可（同样为无签名打包，后续版本会添加代码签名）

## 使用

1. 启动应用后，托盘图标出现
2. 点击托盘图标弹出登录面板
3. 粘贴你的 `Bearer Token`（可从 tokenhub.cash 后台获取）
4. 登录成功后显示用量仪表盘
5. 可调节刷新间隔（1 / 5 / 10 / 30 分钟）

### 获取 API Key

1. 访问 [tokenhub.cash](https://tokenhub.cash) 并登录
2. 进入 API 管理页面
3. 复制你的 Personal Access Token
4. 粘贴到 TokenHub Monitor 的登录输入框中

> 提示：支持粘贴完整的 `Authorization` 请求头（如 `Bearer sk-xxx...`），程序会自动提取 Token 值。

## 开发

```bash
# 安装前端依赖
pnpm install

# 开发模式
pnpm tauri dev

# 构建发布包（macOS）
pnpm tauri build --target aarch64-apple-darwin

# 构建发布包（Windows，需在 Windows 环境）
pnpm tauri build
```

## 前置要求

- macOS 11.0+ / Windows 11
- Node.js + pnpm
- Rust toolchain（rustup）
- macOS：Xcode Command Line Tools
- Windows：Microsoft Visual Studio Build Tools（`cargo build` 需要）

### macOS 托盘行为说明

在 macOS 上，给 `TrayIconBuilder` 挂载 `Menu` 会导致左键点击永远弹出菜单，即使设置 `show_menu_on_left_click(false)` 也无效，`on_tray_icon_event` 的 Left 事件不会触发。因此本应用**不使用托盘菜单**，所有交互（弹出面板、退出）均通过左键点击托盘图标或 UI 内按钮完成。
