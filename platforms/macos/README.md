# Extractr — macOS 适配层

## 架构

```
platforms/macos/
├── Extractr.xcodeproj/       # Xcode 工程配置
├── Extractr/
│   ├── App/                  # SwiftUI 主入口
│   │   ├── ExtractrApp.swift
│   │   ├── ContentView.swift
│   │   └── Info.plist
│   ├── Bridge/               # archive-core FFI 桥接
│   │   ├── CoreBridge.swift  # 调用 Rust cdylib
│   │   └── bridge.h          # C 头文件（cbindgen 生成）
│   ├── Assets.xcassets/      # 图标资源
│   │   └── AppIcon.appiconset/
│   └── Base.lproj/           # 本地化
├── archive-core-sys/         # Rust cdylib 编译脚本
│   ├── Cargo.toml
│   └── build.rs
└── README.md
```

## 核心复用策略

1. **archive-core** 编译为 `cdylib`（C 动态库），macOS 通过 FFI 调用
2. Swift 侧 `CoreBridge` 封装 C 函数为 async/await API
3. UI 用 SwiftUI 原生实现，复用 archive-core 的全部解压/压缩/转换逻辑
4. 进度回调通过 C 函数指针 → Swift closure 桥接

## 构建步骤

```bash
# 1. 编译 archive-core 为 dylib
cd platforms/macos/archive-core-sys
cargo build --release

# 2. 用 cbindgen 生成 C 头文件
cbindgen --crate archive-core -o ../Extractr/Bridge/bridge.h

# 3. Xcode 打开工程编译
open ../Extractr.xcodeproj
```

## 文件关联

macOS 通过 `Info.plist` 的 `CFBundleDocumentTypes` 注册：
- UTType: public.archive, com.pkware.zip-archive, org.7-zip.7z-archive 等
- 右键菜单通过 `NSServices` 或 Quick Look 扩展实现

## 状态

⚠️ 骨架已创建，待实现。核心 Rust 库已就绪，需完成：
- [ ] FFI 导出函数（C ABI）
- [ ] Swift Bridge 封装
- [ ] SwiftUI 界面
- [ ] 文件关联注册
- [ ] 拖拽支持（NSItemProvider）
