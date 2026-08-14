; Extractr NSIS 安装钩子：注册文件关联、图标、右键菜单。
; 安装模式为 currentUser，所有项写入 HKCU，卸载时由 PreUninstall 钩子清理。
; 右键菜单策略：
;   - 压缩包文件（.zip/.7z/...）：右键显示「用 Extractr 打开」
;   - 普通文件：右键显示「用 Extractr 压缩」（用 AppliesTo 排除归档扩展名）
;   - 文件夹：右键显示「用 Extractr 压缩」
; Windows 优先级：HKCU\Software\Classes\*\shell 的默认行为会盖过
; HKCU\Software\Classes\SystemFileAssociations\{ext}\shell，所以必须用 AppliesTo
; 把 `*` 通配的"压缩"作用范围限制为"非归档扩展名"，让压缩包走"打开"分支。
; Windows 图标查找顺序：User Customized > ProgID(扩展名 default) > SystemFileAssociations
;   > * > AllFilesystemObjects。要让资源管理器显示 Extractr 图标，必须把扩展名
;   (default) 指到 Extractr${EXT} ProgID（其 DefaultIcon → Extractr.exe,0）。
; 卸载时还原原值（备份到 ${EXT}_backup）：防止污染系统默认扩展名 ProgID。
; 兼容清理：卸载时仍删旧版的 ExtractrHere/ExtractrSubdir/ExtractrOpen/ExtractrCompress。

; AppliesTo 过滤表达式：排除所有归档扩展名，确保压缩包只走"打开"分支。
; 格式遵循 Windows shell 谓词语法（Advanced Query Syntax，IItemFilter）。
; 资源管理器在每次显示菜单时用该表达式对目标文件求值；不匹配则隐藏菜单项。
; 关键点：
;   - 谓词用 `System.FileExtension:=.ext`（前缀 `:` + 比较符 `=` + 不带引号的扩展名）
;   - 多个值用 OR 串接，外面用 NOT (...) 整体取反
;   - 整个表达式放在 `*` 通配下，必须排除所有归档扩展名，否则 `*` 菜单会盖过
;     SystemFileAssociations 下的"打开"。
!define EXTR_NON_ARCHIVE_FILTER `NOT (System.FileExtension:=.zip OR System.FileExtension:=.7z OR System.FileExtension:=.rar OR System.FileExtension:=.tar OR System.FileExtension:=.gz OR System.FileExtension:=.gzip OR System.FileExtension:=.bz2 OR System.FileExtension:=.xz OR System.FileExtension:=.zst OR System.FileExtension:=.zstd OR System.FileExtension:=.tgz OR System.FileExtension:=.tbz OR System.FileExtension:=.tbz2 OR System.FileExtension:=.txz OR System.FileExtension:=.tzst OR System.FileExtension:=.tzs)`

; ===== 顶层宏：单压缩包扩展名关联 + 图标 + 右键"用 Extractr 打开" =====
; Shell 查找图标顺序：User Customized > ProgID(扩展名 default) > SystemFileAssociations > \* > AllFilesystemObjects
; 我们要让资源管理器显示 Extractr 图标，必须把扩展名 (default) 指到 Extractr${EXT} ProgID
; (该 ProgID 已配置 DefaultIcon → Extractr.exe,0)。
; 安装时先备份原值到 ${EXT}_backup，便于卸载还原；备份键不存在时跳过备份（首次安装）。
; 卸载时：
;   - 备份键存在 → 恢复 backup 值作为 (default)，删除 backup 键
;   - 备份键不存在 → 表示安装前 (default) 为空，直接删除整个 ${EXT} 节点
!macro EXTR_REG_EXT EXT
  ; 备份原 default（若尚未备份）。第一次安装时 backup 不存在，写入 backup；
  ; 重复安装时 backup 已存在则跳过，避免覆盖。
  ReadRegStr $0 HKCU "Software\Classes\${EXT}" "${EXT}_backup"
  ${If} $0 == ""
    ReadRegStr $0 HKCU "Software\Classes\${EXT}" ""
    WriteRegStr HKCU "Software\Classes\${EXT}" "${EXT}_backup" "$0"
  ${EndIf}
  ; 设为 Extractr ProgID（Shell 据此读取 DefaultIcon 显示 Extractr 图标）
  WriteRegStr HKCU "Software\Classes\${EXT}" "" "Extractr${EXT}"
  ; ProgID 默认值 + DefaultIcon + open 动词
  WriteRegStr HKCU "Software\Classes\Extractr${EXT}" "" "Extractr Archive"
  WriteRegStr HKCU "Software\Classes\Extractr${EXT}\DefaultIcon" "" "$INSTDIR\Extractr.exe,0"
  WriteRegStr HKCU "Software\Classes\Extractr${EXT}\shell\open\command" "" '"$INSTDIR\Extractr.exe" "%1"'
  ; OpenWithProgids：把 Extractr${EXT} 加入"打开方式"候选列表
  WriteRegStr HKCU "Software\Classes\${EXT}\OpenWithProgids" "Extractr${EXT}" ""
  ; SystemFileAssociations 下的右键"用 Extractr 打开"（低优先级，但备用）
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrAction" "" "用 Extractr 打开"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrAction" "Position" "Top"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrAction" "Icon" "$INSTDIR\Extractr.exe"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrAction\command" "" '"$INSTDIR\Extractr.exe" "%1"'
!macroend

!macro EXTR_UNREG_EXT EXT
  ; 兼容删旧版动作。
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrHere"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrSubdir"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrOpen"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\ExtractrAction"
  ; 恢复 (default)：若备份键存在则还原原值；否则删除整个 ${EXT} 节点
  ReadRegStr $0 HKCU "Software\Classes\${EXT}" "${EXT}_backup"
  ${If} $0 == ""
    ; 无备份：删 HKCU\Software\Classes\${EXT} 下 Extractr 引用 + OpenWithProgids
    DeleteRegValue HKCU "Software\Classes\${EXT}\OpenWithProgids" "Extractr${EXT}"
  ${Else}
    WriteRegStr HKCU "Software\Classes\${EXT}" "" "$0"
    DeleteRegValue HKCU "Software\Classes\${EXT}" "${EXT}_backup"
  ${EndIf}
  ; 清理 ProgID
  DeleteRegKey HKCU "Software\Classes\Extractr${EXT}"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; 压缩包扩展名：右键「用 Extractr 解压」（与 lib.rs::archive_exts 保持一致）
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
  !insertmacro EXTR_REG_EXT ".tbz"
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

  ; 普通文件右键：用 Extractr 压缩（压缩包扩展名通过 AppliesTo 排除，走"解压"分支）
  WriteRegStr HKCU "Software\Classes\*\shell\ExtractrAction" "" "用 Extractr 压缩"
  WriteRegStr HKCU "Software\Classes\*\shell\ExtractrAction" "Position" "Top"
  WriteRegStr HKCU "Software\Classes\*\shell\ExtractrAction" "Icon" "$INSTDIR\Extractr.exe"
  ; AppliesTo：排除归档扩展名，避免与 SystemFileAssociations 下的"打开"冲突。
  ; Windows 资源管理器默认会把 HKCU\* 的菜单优先于 SystemFileAssociations，
  ; 所以必须显式过滤，否则 1.zip 也会看到"用 Extractr 压缩"。
  WriteRegStr HKCU "Software\Classes\*\shell\ExtractrAction" "AppliesTo" "${EXTR_NON_ARCHIVE_FILTER}"
  WriteRegStr HKCU "Software\Classes\*\shell\ExtractrAction\command" "" '"$INSTDIR\Extractr.exe" "--compress" "%1"'

  ; 通知资源管理器刷新图标和文件关联缓存。
  ; SHCNE_ASSOCCHANGED (0x08000000) 告知 shell 有 ProgID/扩展名关联变更，
  ; 强制资源管理器重新查询 DefaultIcon 并更新显示。
  System::Call 'shell32.dll::SHChangeNotify(i 0x08000000, i 0, i 0, i 0)'

  ; 强制刷新并重建图标缓存，确保已解压/压缩过的文件显示最新 Extractr 图标。
  ; ie4uinit -ClearIconCache 清除内存中的图标缓存；删除 IconCache.db 清除磁盘缓存。
  ExecWait '"$LOCALAPPDATA\IconCache.db" /f'
  nsExec::ExecToLog 'ie4uinit.exe -ClearIconCache'
  System::Call 'shell32.dll::SHChangeNotify(i 0x08000000, i 0, i 0, i 0)'
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
  !insertmacro EXTR_UNREG_EXT ".tbz"
  !insertmacro EXTR_UNREG_EXT ".tbz2"
  !insertmacro EXTR_UNREG_EXT ".txz"
  !insertmacro EXTR_UNREG_EXT ".tzst"
  !insertmacro EXTR_UNREG_EXT ".tzs"

  DeleteRegKey HKCU "Software\Classes\Directory\Background\shell\ExtractrOpen"
  DeleteRegKey HKCU "Software\Classes\Directory\shell\ExtractrAction"
  DeleteRegKey HKCU "Software\Classes\*\shell\ExtractrAction"

  ; 通知资源管理器刷新图标和文件关联缓存。
  System::Call 'shell32.dll::SHChangeNotify(i 0x08000000, i 0, i 0, i 0)'
!macroend
