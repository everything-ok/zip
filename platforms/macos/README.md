# Extractr — macOS (Tauri)

macOS 版本与 Windows 共享同一 Tauri 工程源码，无需单独维护 UI 或核心逻辑。

## 架构

```
项目根目录/
├── src/                    # React 前端（跨平台共享）
├── src-tauri/              # Tauri Rust 后端（跨平台共享）
│   ├── src/                #   commands / state / lib（无 cfg 分支）
│   ├── icons/icon.icns     #   macOS 图标
│   └── tauri.conf.json     #   bundle.macOS 段 + fileAssociations + UTType
├── crates/archive-core/    # 纯 Rust 压缩/解压引擎（跨平台共享）
└── platforms/macos/        # 本目录：macOS 构建说明
```

## 构建步骤

### 前置条件

- macOS 13+ (Ventura)
- Xcode Command Line Tools: `xcode-select --install`
- Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Node.js 18+: `brew install node`

### 构建

```bash
# 在项目根目录
npm install
npm run tauri build
```

产物位于 `src-tauri/target/release/bundle/`:
- `dmg/Extractr_0.2.5_aarch64.dmg` (Apple Silicon)
- `dmg/Extractr_0.2.5_x64.dmg` (Intel)
- `macos/Extractr.app` (独立 .app bundle)

### 开发模式

```bash
npm run tauri dev
```

## 文件关联

macOS 文件关联通过 `tauri.conf.json` 的 `bundle.macOS.infoPlist` 声明:

- `CFBundleDocumentTypes`: 注册为 ZIP/7z/RAR/TAR/GZIP/BZIP2/XZ/Zstd 的查看器
- `UTImportedTypeDeclarations`: 声明非系统 UTType (7z/RAR/XZ/Zstd/BZIP2)
- `LSHandlerRank: Alternate`: 不抢占系统默认，用户可通过 Finder "显示简介"手动设为默认

## macOS 特有行为

| 功能 | Windows | macOS |
|------|---------|-------|
| 文件关联启动 | argv (`%1`) | Apple Events (`RunEvent::Opened`) |
| 右键菜单 | NSIS 注册表 | Finder "打开方式" (系统自动) |
| 设为默认引导 | 显示引导条 | 隐藏 (macOS 通过 Finder 设置) |
| 默认应用设置 | `ms-settings:defaultapps` | `x-apple.systempreferences:` |
| WebView | WebView2 (自动安装) | WKWebView (系统内置) |

## 签名与公证 (发布用)

```bash
# 需 Apple Developer 证书
export APPLE_SIGNING_IDENTITY="Developer ID Application: ..."
export APPLE_ID="your@apple.id"
export APPLE_PASSWORD="app-specific-password"
export APPLE_TEAM_ID="TEAMID"

npm run tauri build
# Tauri 自动签名 + 公证
```

未签名 .app 可本地使用，但 Gatekeeper 会拦截分发。
