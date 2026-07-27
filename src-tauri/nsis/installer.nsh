; Extractr NSIS 安装钩子：注册"用 Extractr 打开"右键菜单与文件关联图标。
; 安装模式为 currentUser，所有项写入 HKCU，卸载时由 PreUninstall 钩子清理。

!macro NSIS_HOOK_POSTINSTALL
  ; 可执行文件完整路径。
  ReadEnvStr $0 "COMSPEC"
  ; 右键菜单命令：对文件、目录、目录背景均提供"用 Extractr 打开"。
  WriteRegStr HKCU "Software\Classes\*\shell\Extractr" "" "用 Extractr 打开"
  WriteRegStr HKCU "Software\Classes\*\shell\Extractr" "Icon" "$INSTDIR\Extractr.exe"
  WriteRegStr HKCU "Software\Classes\*\shell\Extractr\command" "" '"$INSTDIR\Extractr.exe" "%1"'

  WriteRegStr HKCU "Software\Classes\Directory\shell\Extractr" "" "用 Extractr 打开"
  WriteRegStr HKCU "Software\Classes\Directory\shell\Extractr" "Icon" "$INSTDIR\Extractr.exe"
  WriteRegStr HKCU "Software\Classes\Directory\shell\Extractr\command" "" '"$INSTDIR\Extractr.exe" "%1"'

  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\Extractr" "" "用 Extractr 打开"
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\Extractr" "Icon" "$INSTDIR\Extractr.exe"
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\Extractr\command" "" '"$INSTDIR\Extractr.exe" "%V"'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; 卸载时清理所有注册的右键菜单项。
  DeleteRegKey HKCU "Software\Classes\*\shell\Extractr"
  DeleteRegKey HKCU "Software\Classes\Directory\shell\Extractr"
  DeleteRegKey HKCU "Software\Classes\Directory\Background\shell\Extractr"
!macroend
