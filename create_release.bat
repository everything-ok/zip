@echo off
REM Extractr v0.2.5 Release Publisher
REM Usage: Set GITHUB_TOKEN env var first, then run this script

REM Check for GitHub token
if "%GITHUB_TOKEN%"=="" (
    echo ERROR: Set GITHUB_TOKEN environment variable first.
    echo Example: set GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxx
    echo Then run: create_release.bat
    pause
    exit /b 1
)

echo Logging into GitHub CLI...
gh auth login --hostname github.com --token %GITHUB_TOKEN%

if %ERRORLEVEL% neq 0 (
    echo GitHub login failed.
    pause
    exit /b 1
)

echo Creating GitHub Release for Extractr v0.2.5...

gh release create v0.2.5 ^
    --repo everything-ok/zip ^
    --title "Extractr v0.2.5" ^
    --notes "## Extractr v0.2.5 Release

## 支持平台
- **Windows 10** (x64) - 推荐
- **Windows 11** (x64) - 兼容

## 安装包
- **NSIS 安装包** (推荐): Extractr_0.2.5_x64-setup.exe - 一键安装，自动注册文件关联和右键菜单
- **MSI 英文版**: Extractr_0.2.5_x64_en-US.msi - 适用于英文 Windows，企业部署
- **MSI 简体中文版**: Extractr_0.2.5_x64_zh-CN.msi - 适用于中文 Windows，企业部署

## 支持的压缩格式
**解压**: ZIP, RAR, 7z, TAR, GZIP, BZIP2, XZ, Zstandard 及所有 .tar.* 组合格式
**创建**: ZIP, 7z, TAR, TAR.GZ, TAR.XZ (均支持密码保护)

## 本次更新
- 图标全面更新 (从 logo.svg 重新生成所有尺寸)
- 移除顶部栏 Logo 展示
- 移除关于弹窗 Logo 展示
- 修复安装时旧版右键菜单残留问题
- 安装时自动清理图标缓存

## 系统要求
- Windows 10/11 x64
- 无需管理员权限 (用户级安装)
- WebView2 Runtime (未安装时自动下载)
" ^
    --target main ^
    "src-tauri\target\release\bundle\nsis\Extractr_0.2.5_x64-setup.exe" ^
    "src-tauri\target\release\bundle\msi\Extractr_0.2.5_x64_en-US.msi" ^
    "src-tauri\target\release\bundle\msi\Extractr_0.2.5_x64_zh-CN.msi"

if %ERRORLEVEL% equ 0 (
    echo.
    echo SUCCESS! Release v0.2.5 created at:
    echo https://github.com/everything-ok/zip/releases/tag/v0.2.5
) else (
    echo.
    echo Release creation failed. Check the errors above.
)

pause
