//! 安全输出工具：路径净化（Zip Slip）、Windows 保留名、原子文件提交。
//! 所有 extractor 必须经由本模块解析路径和创建输出，避免归档条目逃逸或
//! 在取消/失败时截断既有文件。

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::bail;

use crate::error::ArchiveError;
use crate::types::OverwritePolicy;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// 校验归档内路径并返回其相对输出路径。
///
/// `dest_root` 保留在接口中以兼容各 extractor；校验本身没有文件系统副作用。
/// 它不创建目录、不规范化已有目录，也不吞掉 I/O 错误。
/// 同时校验总路径长度上限（防超长路径滥用），默认上限见 `ExtractLimits::max_path_len`。
pub fn sanitize_entry_path(entry: &str, _dest_root: &Path) -> anyhow::Result<PathBuf> {
    sanitize_entry_path_limited(entry, _dest_root, 4096)
}

/// 带路径长度上限的净化入口，供 extractor 传入 `ExtractLimits.max_path_len`。
pub fn sanitize_entry_path_limited(
    entry: &str,
    _dest_root: &Path,
    max_path_len: usize,
) -> anyhow::Result<PathBuf> {
    if entry.is_empty() {
        bail!("空路径条目");
    }

    // 归档规范使用 `/`，但也必须将 `\\` 当作路径分隔符来阻断 Windows 路径穿越。
    let normalized = entry.replace('\\', "/");
    let path = Path::new(&normalized);
    if path.is_absolute() || normalized.starts_with('/') {
        bail!("拒绝绝对路径条目: {entry}");
    }

    let mut safe = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(name) => {
                reject_windows_reserved_name(name)?;
                safe.push(name);
            }
            Component::CurDir => {}
            Component::ParentDir => bail!("拒绝包含 .. 的条目: {entry}"),
            Component::RootDir | Component::Prefix(_) => bail!("拒绝绝对路径条目: {entry}"),
        }
    }
    if safe.as_os_str().is_empty() {
        bail!("空路径条目: {entry}");
    }
    let len = safe.to_string_lossy().len();
    if len > max_path_len {
        return Err(ArchiveError::PathTooLong {
            path: entry.to_string(),
            len,
            max: max_path_len,
        }
        .into());
    }
    Ok(safe)
}

/// 创建一个安全目录。每一级既有符号链接都被拒绝，避免借由归档路径进入目标外。
pub fn ensure_safe_directory(dest_root: &Path, relative: &Path) -> anyhow::Result<PathBuf> {
    fs::create_dir_all(dest_root)?;
    let root = dest_root.canonicalize()?;
    let mut current = root;

    for component in relative.components() {
        let Component::Normal(name) = component else {
            bail!("内部错误：未净化的输出路径");
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                bail!("拒绝经过符号链接目录: {}", current.display());
            }
            Ok(metadata) if !metadata.is_dir() => {
                bail!("目标路径不是目录: {}", current.display());
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(&current)?,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(current)
}

/// 根据覆盖策略决定最终文件路径。`target` 必须已经是安全的目标路径。
/// 返回 `None` 表示应跳过条目。
pub fn resolve_target_path(
    target: &Path,
    overwrite: OverwritePolicy,
) -> anyhow::Result<Option<PathBuf>> {
    if !target.exists() {
        return Ok(Some(target.to_path_buf()));
    }
    match overwrite {
        OverwritePolicy::Skip => Ok(None),
        OverwritePolicy::Overwrite => Ok(Some(target.to_path_buf())),
        OverwritePolicy::Error => bail!("目标已存在: {}", target.display()),
        OverwritePolicy::Rename => {
            let parent = target.parent().unwrap_or_else(|| Path::new("."));
            let stem = target
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("file");
            let extension = target.extension();
            for index in 1..=9999_u32 {
                let mut name = format!("{stem} ({index})");
                if let Some(extension) = extension {
                    name.push('.');
                    name.push_str(&extension.to_string_lossy());
                }
                let candidate = parent.join(name);
                if !candidate.exists() {
                    return Ok(Some(candidate));
                }
            }
            bail!("重命名冲突过多: {}", target.display());
        }
    }
}

/// 在目标文件同目录创建临时文件。写入成功后调用 `commit`；取消或出错时 Drop
/// 会删除临时文件，因此不会破坏既有目标文件。
pub struct AtomicOutput {
    file: Option<File>,
    temp_path: PathBuf,
    target_path: PathBuf,
    committed: bool,
}

impl AtomicOutput {
    pub fn file_mut(&mut self) -> &mut File {
        self.file.as_mut().expect("AtomicOutput 文件已提交")
    }

    /// 刷新并将临时文件替换为最终文件。Windows 不允许 `rename` 覆盖既有文件，
    /// 所以先把旧文件移至同目录备份；若临时文件提交失败，立即恢复旧文件。
    pub fn commit(mut self) -> anyhow::Result<PathBuf> {
        if let Some(file) = self.file.as_mut() {
            file.flush()?;
            file.sync_all()?;
        }
        drop(self.file.take());

        let backup_path = if self.target_path.exists() {
            Some(unique_sibling_path(&self.target_path, "backup")?)
        } else {
            None
        };
        if let Some(backup_path) = &backup_path {
            fs::rename(&self.target_path, backup_path)?;
        }
        if let Err(error) = fs::rename(&self.temp_path, &self.target_path) {
            if let Some(backup_path) = &backup_path {
                let _ = fs::rename(backup_path, &self.target_path);
            }
            return Err(error.into());
        }
        if let Some(backup_path) = backup_path {
            fs::remove_file(backup_path)?;
        }
        self.committed = true;
        Ok(self.target_path.clone())
    }
}

impl Drop for AtomicOutput {
    fn drop(&mut self) {
        if !self.committed {
            let _ = self.file.take();
            let _ = fs::remove_file(&self.temp_path);
        }
    }
}

/// 解析条目、应用覆盖策略、创建安全父目录，并返回原子输出文件。
pub fn prepare_output(
    dest_root: &Path,
    entry_path: &str,
    overwrite: OverwritePolicy,
) -> anyhow::Result<Option<AtomicOutput>> {
    let relative = sanitize_entry_path(entry_path, dest_root)?;
    let parent_relative = relative.parent().unwrap_or_else(|| Path::new(""));
    let parent = ensure_safe_directory(dest_root, parent_relative)?;
    let target = dest_root.join(&relative);
    let Some(target) = resolve_target_path(&target, overwrite)? else {
        return Ok(None);
    };

    for _ in 0..128 {
        let temp_path = unique_sibling_path_in(&parent, &target, "tmp")?;
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => {
                return Ok(Some(AtomicOutput {
                    file: Some(file),
                    temp_path,
                    target_path: target,
                    committed: false,
                }));
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    bail!("无法创建唯一临时文件: {}", target.display())
}

fn unique_sibling_path(target: &Path, label: &str) -> anyhow::Result<PathBuf> {
    let parent = target
        .parent()
        .ok_or_else(|| anyhow::anyhow!("目标文件没有父目录"))?;
    unique_sibling_path_in(parent, target, label)
}

fn unique_sibling_path_in(parent: &Path, target: &Path, label: &str) -> anyhow::Result<PathBuf> {
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("无法解析目标文件名: {}", target.display()))?;
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    Ok(parent.join(format!(
        ".{name}.extractr-{}-{sequence}.{label}",
        std::process::id()
    )))
}

/// 拒绝 Windows 保留设备名和非法字符。
fn reject_windows_reserved_name(name: &std::ffi::OsStr) -> anyhow::Result<()> {
    let Some(name) = name.to_str() else {
        bail!("归档路径包含非 UTF-8 文件名");
    };
    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    const RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if RESERVED.contains(&stem.as_str()) {
        bail!("拒绝 Windows 保留名: {name}");
    }
    if name
        .chars()
        .any(|character| matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
    {
        bail!("拒绝含非法字符的文件名: {name}");
    }
    if name.chars().any(|character| (character as u32) < 0x20) {
        bail!("拒绝含控制字符的文件名: {name}");
    }
    if name.len() > 255 {
        bail!("文件名过长: {name}");
    }
    Ok(())
}
