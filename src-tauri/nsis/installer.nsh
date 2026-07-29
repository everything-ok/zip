; Extractr NSIS 安装钩子：注册文件关联、右键菜单。
; 安装模式为 currentUser，所有项写入 HKCU，卸载时由 PreUninstall 钩子清理。
; 右键动作通过 argv 传动作参数：--extract-here / --extract-to-subdir / 裸路径（打开）。

; ===== 顶层宏：单个扩展名的右键三动作注册（避免宏内定义宏）=====
!macro EXTR_REG_EXT EXT
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrHere" "" "解压到此处"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrHere" "Position" "Top"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrHere" "Icon" "$INSTDIR\Extractr.exe"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrHere\command" "" '"$INSTDIR\Extractr.exe" "--extract-here" "%1"'

  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrSubdir" "" "解压到同名子目录"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrSubdir" "Position" "Top"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrSubdir" "Icon" "$INSTDIR\Extractr.exe"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrSubdir\command" "" '"$INSTDIR\Extractr.exe" "--extract-to-subdir" "%1"'

  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrOpen" "" "用 Extractr 打开"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrOpen" "Position" "Top"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrOpen" "Icon" "$INSTDIR\Extractr.exe"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrOpen\command" "" '"$INSTDIR\Extractr.exe" "%1"'
!macroend

!macro EXTR_UNREG_EXT EXT
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
!macroend
