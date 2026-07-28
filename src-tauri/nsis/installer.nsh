; Extractr NSIS 安装钩子：注册右键菜单与文件关联。
; 安装模式为 currentUser，所有项写入 HKCU，卸载时由 PreUninstall 钩子清理。
; 右键动作通过 argv 传动作参数：--extract-here / --extract-to-subdir / --extract-to。

!macro NSIS_HOOK_POSTINSTALL
  ; ===== 右键菜单三动作：解压到此处 / 解压到子目录 / 解压到… =====
  ; 对文件（*）：
  WriteRegStr HKCU "Software\Classes\*\shell\ExtractrHere" "" "解压到此处"
  WriteRegStr HKCU "Software\Classes\*\shell\ExtractrHere" "Icon" "$INSTDIR\Extractr.exe"
  WriteRegStr HKCU "Software\Classes\*\shell\ExtractrHere\command" "" '"$INSTDIR\Extractr.exe" "--extract-here" "%1"'

  WriteRegStr HKCU "Software\Classes\*\shell\ExtractrSubdir" "" "解压到同名子目录"
  WriteRegStr HKCU "Software\Classes\*\shell\ExtractrSubdir" "Icon" "$INSTDIR\Extractr.exe"
  WriteRegStr HKCU "Software\Classes\*\shell\ExtractrSubdir\command" "" '"$INSTDIR\Extractr.exe" "--extract-to-subdir" "%1"'

  WriteRegStr HKCU "Software\Classes\*\shell\ExtractrTo" "" "用 Extractr 打开"
  WriteRegStr HKCU "Software\Classes\*\shell\ExtractrTo" "Icon" "$INSTDIR\Extractr.exe"
  WriteRegStr HKCU "Software\Classes\*\shell\ExtractrTo\command" "" '"$INSTDIR\Extractr.exe" "%1"'

  ; 对目录（Directory）：在目录上右键时解压到该目录
  WriteRegStr HKCU "Software\Classes\Directory\shell\ExtractrTo" "" "用 Extractr 打开"
  WriteRegStr HKCU "Software\Classes\Directory\shell\ExtractrTo" "Icon" "$INSTDIR\Extractr.exe"
  WriteRegStr HKCU "Software\Classes\Directory\shell\ExtractrTo\command" "" '"$INSTDIR\Extractr.exe" "%1"'

  ; 对目录背景（Directory\Background）：用当前目录作为目标
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\ExtractrTo" "" "用 Extractr 打开"
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\ExtractrTo" "Icon" "$INSTDIR\Extractr.exe"
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\ExtractrTo\command" "" '"$INSTDIR\Extractr.exe" "%V"'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; 卸载时清理所有注册的右键菜单项。
  DeleteRegKey HKCU "Software\Classes\*\shell\ExtractrHere"
  DeleteRegKey HKCU "Software\Classes\*\shell\ExtractrSubdir"
  DeleteRegKey HKCU "Software\Classes\*\shell\ExtractrTo"
  DeleteRegKey HKCU "Software\Classes\Directory\shell\ExtractrTo"
  DeleteRegKey HKCU "Software\Classes\Directory\Background\shell\ExtractrTo"
!macroend
