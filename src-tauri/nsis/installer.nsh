; Extractr NSIS 安装钩子：注册文件关联、右键菜单。
; 安装模式为 currentUser，所有项写入 HKCU，卸载时由 PreUninstall 钩子清理。
; 右键菜单策略：
;   - 压缩包文件（.zip/.7z/...）：右键显示「用 Extractr 解压」
;   - 普通文件：右键显示「用 Extractr 压缩」（通过 * 通配注册）
;   - 文件夹：右键显示「用 Extractr 压缩」
; Windows 优先级：SystemFileAssociations\{ext} > *，压缩包扩展名覆盖通配的"压缩"为"解压"。
; 兼容清理：卸载时仍删旧版的 ExtractrHere/ExtractrSubdir/ExtractrOpen/ExtractrCompress。

; ===== 顶层宏：单压缩包扩展名右键"用 Extractr 解压" =====
!macro EXTR_REG_EXT EXT
  ; 右键菜单：用 Extractr 打开（覆盖 * 通配的"压缩"）
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrAction" "" "用 Extractr 打开"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrAction" "Position" "Top"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrAction" "Icon" "$INSTDIR\Extractr.exe"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrAction\command" "" '"$INSTDIR\Extractr.exe" "%1"'
  ; ProgID + DefaultIcon：让资源管理器在压缩包文件上显示 Extractr 图标
  WriteRegStr HKCU "Software\Classes\Extractr${EXT}" "" "Extractr Archive"
  WriteRegStr HKCU "Software\Classes\Extractr${EXT}\DefaultIcon" "" "$INSTDIR\Extractr.exe,0"
  WriteRegStr HKCU "Software\Classes\Extractr${EXT}\shell\open\command" "" '"$INSTDIR\Extractr.exe" "%1"'
  WriteRegStr HKCU "Software\Classes\${EXT}\OpenWithProgids" "Extractr${EXT}" ""
!macroend

!macro EXTR_UNREG_EXT EXT
  ; 兼容删旧版动作。
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrHere"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrSubdir"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrOpen"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrAction"
  ; 清理 ProgID + OpenWithProgids
  DeleteRegKey HKCU "Software\Classes\Extractr${EXT}"
  DeleteRegValue HKCU "Software\Classes\${EXT}\OpenWithProgids" "Extractr${EXT}"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; 压缩包扩展名：右键「用 Extractr 解压」
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

  ; 目录背景右键：用 Extractr 打开
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\ExtractrOpen" "" "用 Extractr 打开"
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\ExtractrOpen" "Icon" "$INSTDIR\Extractr.exe"
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\ExtractrOpen\command" "" '"$INSTDIR\Extractr.exe" "%V"'

  ; 文件夹右键：用 Extractr 压缩
  WriteRegStr HKCU "Software\Classes\Directory\shell\ExtractrAction" "" "用 Extractr 压缩"
  WriteRegStr HKCU "Software\Classes\Directory\shell\ExtractrAction" "Position" "Top"
  WriteRegStr HKCU "Software\Classes\Directory\shell\ExtractrAction" "Icon" "$INSTDIR\Extractr.exe"
  WriteRegStr HKCU "Software\Classes\Directory\shell\ExtractrAction\command" "" '"$INSTDIR\Extractr.exe" "--compress" "%1"'

  ; 普通文件右键：用 Extractr 压缩（压缩包扩展名通过 SystemFileAssociations 覆盖为"解压"）
  WriteRegStr HKCU "Software\Classes\*\shell\ExtractrAction" "" "用 Extractr 压缩"
  WriteRegStr HKCU "Software\Classes\*\shell\ExtractrAction" "Position" "Top"
  WriteRegStr HKCU "Software\Classes\*\shell\ExtractrAction" "Icon" "$INSTDIR\Extractr.exe"
  WriteRegStr HKCU "Software\Classes\*\shell\ExtractrAction\command" "" '"$INSTDIR\Extractr.exe" "--compress" "%1"'
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
  DeleteRegKey HKCU "Software\Classes\Directory\shell\ExtractrAction"
  DeleteRegKey HKCU "Software\Classes\*\shell\ExtractrAction"
!macroend
