; Extractr NSIS 安装钩子：注册文件关联、右键菜单。
; 安装模式为 currentUser，所有项写入 HKCU，卸载时由 PreUninstall 钩子清理。
; 右键只保留"用 Extractr 打开"：打开后在 UI 中配置目标/覆盖策略/密码，最灵活。
; 兼容清理：卸载时仍删旧版的 ExtractrHere/ExtractrSubdir（v0.2.1 前注册过）。

; ===== 顶层宏：单扩展名右键"用 Extractr 打开" =====
!macro EXTR_REG_EXT EXT
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrOpen" "" "用 Extractr 打开"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrOpen" "Position" "Top"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrOpen" "Icon" "$INSTDIR\Extractr.exe"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrOpen\command" "" '"$INSTDIR\Extractr.exe" "%1"'
!macroend

!macro EXTR_UNREG_EXT EXT
  ; 兼容删旧版三动作。
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrHere"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrSubdir"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrOpen"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro EXTR_REG_EXT ".zip"
  !insertmacro EXTR_REG_EXT ".7z"
  !insertmacro EXTR_REG_EXT ".rar"
  !insertmacro EXTR_REG_EXT ".tar"
  !insertmacro EXTR_REG_EXT ".gz"
  !insertmacro EXTR_REG_EXT ".gzip"
  !insertmacro EXTR_REG_EXT ".bz2"
  !insertmacro EXTR_REG_EXT ".xz"
  !insertmacro EXTR_REG_EXT ".zst"
  !insertmacro EXTR_REG_EXT ".zstd"
  !insertmacro EXTR_REG_EXT ".tgz"
  !insertmacro EXTR_REG_EXT ".tbz2"
  !insertmacro EXTR_REG_EXT ".txz"
  !insertmacro EXTR_REG_EXT ".tzst"
  !insertmacro EXTR_REG_EXT ".tzs"

  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\ExtractrOpen" "" "用 Extractr 打开"
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\ExtractrOpen" "Icon" "$INSTDIR\Extractr.exe"
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\ExtractrOpen\command" "" '"$INSTDIR\Extractr.exe" "%V"'

  ; 文件夹右键：用 Extractr 压缩（预填该文件夹为源）
  WriteRegStr HKCU "Software\Classes\Directory\shell\ExtractrCompress" "" "用 Extractr 压缩"
  WriteRegStr HKCU "Software\Classes\Directory\shell\ExtractrCompress" "Position" "Top"
  WriteRegStr HKCU "Software\Classes\Directory\shell\ExtractrCompress" "Icon" "$INSTDIR\Extractr.exe"
  WriteRegStr HKCU "Software\Classes\Directory\shell\ExtractrCompress\command" "" '"$INSTDIR\Extractr.exe" "--compress" "%1"'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro EXTR_UNREG_EXT ".zip"
  !insertmacro EXTR_UNREG_EXT ".7z"
  !insertmacro EXTR_UNREG_EXT ".rar"
  !insertmacro EXTR_UNREG_EXT ".tar"
  !insertmacro EXTR_UNREG_EXT ".gz"
  !insertmacro EXTR_UNREG_EXT ".gzip"
  !insertmacro EXTR_UNREG_EXT ".bz2"
  !insertmacro EXTR_UNREG_EXT ".xz"
  !insertmacro EXTR_UNREG_EXT ".zst"
  !insertmacro EXTR_UNREG_EXT ".zstd"
  !insertmacro EXTR_UNREG_EXT ".tgz"
  !insertmacro EXTR_UNREG_EXT ".tbz2"
  !insertmacro EXTR_UNREG_EXT ".txz"
  !insertmacro EXTR_UNREG_EXT ".tzst"
  !insertmacro EXTR_UNREG_EXT ".tzs"

  DeleteRegKey HKCU "Software\Classes\Directory\Background\shell\ExtractrOpen"
  DeleteRegKey HKCU "Software\Classes\Directory\shell\ExtractrCompress"
!macroend
