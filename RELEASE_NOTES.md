# Extractr v0.2.5 GitHub Release 创建指南

## 方式一：浏览器授权 gh CLI（推荐）

在 **PowerShell / CMD** 中运行以下命令：

```powershell
gh auth login
```

按提示选择：
- GitHub.com
- HTTPS
- Login with web browser
- 复制代码并在浏览器中授权

授权成功后运行：

```powershell
gh release create v0.2.5 --repo everything-ok/zip --title "Extractr v0.2.5" --notes-file RELEASE_NOTES.md
```

## 方式二：提供 GitHub Token

如果你有 GitHub Personal Access Token（需要 `repo` 权限），设置环境变量后运行 `create_release.bat`：

```powershell
$env:GITHUB_TOKEN = "ghp_xxxxxxxxxxxxxxxxxxxx"
.\create_release.bat
```

## 方式三：GitHub 网页手动创建

1. 打开 https://github.com/everything-ok/zip/releases/new
2. Tag version: `v0.2.5`
3. Target: `main`
4. Title: `Extractr v0.2.5`
5. Description: 见下方 Release 说明
6. 上传 3 个附件：
   - `Extractr_0.2.5_x64-setup.exe` (NSIS 安装包)
   - `Extractr_0.2.5_x64_en-US.msi` (MSI 英文)
   - `Extractr_0.2.5_x64_zh-CN.msi` (MSI 简体中文)

## Release 说明文本

```markdown
## Extractr v0.2.5

### 支持平台
- **Windows 10** (x64) - 推荐
- **Windows 11** (x64) - 兼容

### 安装包
- **NSIS 安装包** (推荐): Extractr_0.2.5_x64-setup.exe — 一键安装，自动注册文件关联和右键菜单
- **MSI 英文版**: Extractr_0.2.5_x64_en-US.msi — 适用于英文 Windows，企业部署
- **MSI 简体中文版**: Extractr_0.2.5_x64_zh-CN.msi — 适用于中文 Windows，企业部署

### 支持的压缩格式
**解压**: ZIP, RAR (含RAR5), 7z, TAR, GZIP, BZIP2, XZ, Zstandard 及所有 .tar.* 组合格式 (.tar.gz / .tar.bz2 / .tar.xz / .tar.zst 等)

**创建**: ZIP, 7z, TAR, TAR.GZ, TAR.XZ（均支持密码保护 AES-256）

### 系统要求
- Windows 10/11 x64
- 无需管理员权限（用户级安装）
- WebView2 Runtime（未安装时自动下载）

### 本次更新 (v0.2.5)
- 图标全面更新（从 logo.svg 重新生成所有尺寸）
- 移除顶部栏 Logo 展示
- 移除关于弹窗 Logo 展示
- 修复安装时旧版右键菜单残留问题
- 安装时自动清理图标缓存
```

## 安装包路径

构建完成后安装包在：

```
C:\Users\Administrator\AppData\Local\Temp\cursor-sandbox-cache\b53ced45072d2bc07831de58cdd1b9a3\cargo-target\release\bundle\
├── nsis\Extractr_0.2.5_x64-setup.exe        (约 2.9 MB)
└── msi\Extractr_0.2.5_x64_en-US.msi        (约 3.7 MB)
    └── msi\Extractr_0.2.5_x64_zh-CN.msi     (约 3.7 MB)
```

直接用文件资源管理器复制出来即可。
