# Extractr — 鸿蒙 (HarmonyOS) 适配层

## 架构

```
platforms/harmony/
├── entry/                        # 鸿蒙应用主模块
│   └── src/main/
│       ├── ets/                  # ArkTS 业务代码
│       │   ├── common/           # 工具函数、常量
│       │   ├── bridge/           # NAPI 桥接层
│       │   ├── entryability/     # UIAbility 入口
│       │   ├── pages/            # 页面
│       │   └── model/            # 数据模型
│       └── resources/            # 资源文件
│           ├── base/element/     # 字符串/颜色资源
│           ├── base/media/       # 图标/图片
│           └── rawfile/          # 原始资源
├── archive-core-napi/            # Rust → NAPI 桥接模块
│   ├── Cargo.toml
│   └── src/lib.rs
└── README.md
```

## 核心复用策略

1. **archive-core** 编译为 HarmonyOS NAPI 原生库（.so）
2. ArkTS 侧通过 `@ohos.napi` 调用 Rust 导出函数
3. UI 用 ArkUI 声明式实现，复用 archive-core 全部逻辑
4. 进度回调通过 NAPI ThreadSafeFunction → ArkTS 回调桥接

## 构建步骤

```bash
# 1. 编译 archive-core 为 NAPI .so
cd platforms/harmony/archive-core-napi
cargo build --target aarch64-unknown-linux-ohos --release

# 2. 将 .so 复制到 entry/src/main/cpp/libs/
cp target/aarch64-unknown-linux-ohos/release/libarchive_core_napi.so \
   ../entry/src/main/cpp/libs/arm64-v8a/

# 3. DevEco Studio 打开工程编译
# 打开 platforms/harmony 目录
```

## 文件关联

鸿蒙通过 `module.json5` 的 `skills` + `uris` 注册：
- mimeType: application/zip, application/x-7z-compressed, application/x-rar-compressed 等
- 支持右键/长按"用 Extractr 打开"

## 状态

⚠️ 骨架已创建，待实现。核心 Rust 库已就绪，需完成：
- [ ] NAPI 导出函数
- [ ] ArkTS Bridge 封装
- [ ] ArkUI 界面
- [ ] 文件关联注册
- [ ] 拖拽/文件选择支持
