# Extractr

一个**低资源占用**的跨平台解压工具。当前目标为 Windows；归档处理核心 `archive-core` 为无 Tauri、无运行时依赖的纯 Rust 库，以便后续复用于 macOS 与 HarmonyOS 的原生适配层。

## 功能

- ZIP、RAR（含 RAR5）、7z、TAR、GZIP、BZIP2、XZ、Zstandard，以及 `.tar.gz` / `.tar.bz2` / `.tar.xz` / `.tar.zst`
- 文件拖拽和原生选择器
- 归档内容预览、密码输入、目标目录选择
- 解压进度、取消任务、任务队列
- 中文 / English 切换，深色 / 浅色主题
- 安全防护：Zip Slip / Windows 设备名拦截、符号链接跳过、256KB 流式缓冲

## 架构

```
crates/archive-core/  纯 Rust 解压核心（跨平台复用）
src-tauri/            Tauri 2 命令、Channel 进度桥接、桌面打包
src/                  React + TypeScript + Tailwind 用户界面
```

## 开发

```bash
# 安装前端依赖
npm install

# 验证 Rust 核心（含 Zip Slip 回归测试）
cargo test -p archive-core

# 启动桌面开发环境
npm run tauri dev

# 生成 Windows 安装包（NSIS + MSI）
npm run tauri build
```

> 首次构建 RAR 支持需安装 Visual Studio C++ Build Tools；项目已使用 `unrar` 的 RARLab UnRAR 源码绑定。

## 打包策略

Windows 端配置了 `downloadBootstrapper`：未安装 WebView2 Runtime 的系统会在安装时自动下载；NSIS 使用 `currentUser` 安装模式，不需要管理员权限。
