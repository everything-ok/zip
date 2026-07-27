//! RAR 解压器（基于 `unrar` crate，C++ 绑定 RARLab 官方源码，支持 RAR5 + 密码）。
//! 仅在启用 `rar` feature 时编译。
//!
//! 安全说明：`unrar` 的 `extract_with_base` 会按条目路径直接写入目标目录，
//! 无法可靠执行覆盖策略。因此采用“隔离解压 → 逐条安全提交”两阶段：
//! 1. 将整个归档解压到目标目录同卷下的唯一隔离临时目录；
//! 2. 对每个条目用 `sanitize_entry_path` + `resolve_target_path` 决策，
//!    再以同卷原子移动提交到最终目录。
//!
//! 取消或失败时清理隔离目录，绝不破坏目标目录既有文件。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::ArchiveError;
use crate::safety::{ensure_safe_directory, resolve_target_path, sanitize_entry_path};
use crate::traits::{ArchiveExtractor, ExtractContext};
use crate::types::{ArchiveEntry, ArchiveFormat, ExtractSummary};

static ISOLATION_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub struct RarExtractor;

impl ArchiveExtractor for RarExtractor {
    fn format_kind(&self) -> ArchiveFormat {
        ArchiveFormat::Rar
    }

    fn supports_password(&self) -> bool {
        true
    }

    fn list(&self, path: &Path, password: Option<&str>) -> anyhow::Result<Vec<ArchiveEntry>> {
        let archive = match password {
            Some(value) => unrar::Archive::with_password(path, value),
            None => unrar::Archive::new(path),
        };
        let mut entries = Vec::new();
        for entry in archive.open_for_listing()? {
            let entry = entry?;
            entries.push(ArchiveEntry {
                path: entry.filename.to_string_lossy().to_string(),
                size: entry.unpacked_size,
                compressed_size: 0,
                is_dir: entry.is_directory(),
                is_encrypted: entry.is_encrypted(),
                modified: None,
            });
        }
        Ok(entries)
    }

    fn extract(&self, ctx: &ExtractContext) -> anyhow::Result<ExtractSummary> {
        fs::create_dir_all(ctx.dest)?;

        // 预扫描：先把所有条目路径校验一遍，遇到逃逸立刻拒绝，不进入解压阶段。
        let preflight = self.preflight(ctx)?;
        // 预扫描已拿到每个条目的未压缩大小，汇总成任务总量。
        let total_bytes = preflight
            .entries
            .iter()
            .filter(|entry| !entry.is_directory)
            .map(|entry| entry.unpacked_size)
            .sum();
        ctx.progress.on_start(preflight.count, total_bytes);

        // 隔离解压目录与目标同卷，保证提交阶段可用原子 rename。
        let isolation = unique_isolation_dir(ctx.dest)?;
        let mut isolation_guard = IsolationGuard::new(isolation.clone());

        let archive = match &ctx.options.password {
            Some(value) => unrar::Archive::with_password(ctx.source, value),
            None => unrar::Archive::new(ctx.source),
        };

        let mut summary = ExtractSummary {
            entries_total: preflight.count,
            ..Default::default()
        };

        // 阶段一：交给 unrar 把归档完整写入隔离目录。密码错误等异常在此暴露。
        match archive.open_for_processing() {
            Ok(mut open) => {
                let mut index = 0_usize;
                while let Some(file) = match open.read_header() {
                    Ok(option) => option,
                    Err(error) => {
                        if looks_like_password_error(&error.to_string()) {
                            anyhow::bail!(ArchiveError::WrongPassword);
                        }
                        return Err(error.into());
                    }
                } {
                    if ctx.cancel.is_cancelled() {
                        summary.cancelled = true;
                        break;
                    }
                    open = file.extract_with_base(&isolation)?;
                    index += 1;
                }
                let _ = index;
            }
            Err(error) => {
                if looks_like_password_error(&error.to_string()) {
                    anyhow::bail!(ArchiveError::WrongPassword);
                }
                return Err(error.into());
            }
        }

        // 阶段二：把隔离目录里的文件逐个安全提交到目标目录。
        match commit_isolation(ctx, &isolation, &preflight, ctx.cancel) {
            Ok(commit_summary) => {
                summary.entries_extracted += commit_summary.entries_extracted;
                summary.entries_skipped += commit_summary.entries_skipped;
                summary.bytes_written += commit_summary.bytes_written;
                if commit_summary.cancelled {
                    summary.cancelled = true;
                }
            }
            Err(error) => {
                let _ = isolation_guard.remove_now();
                return Err(error);
            }
        }

        isolation_guard.commit();
        Ok(summary)
    }
}

impl RarExtractor {
    fn preflight(&self, ctx: &ExtractContext) -> anyhow::Result<Preflight> {
        let archive = match &ctx.options.password {
            Some(value) => unrar::Archive::with_password(ctx.source, value),
            None => unrar::Archive::new(ctx.source),
        };
        let mut entries = Vec::new();
        for header in archive.open_for_listing()? {
            if ctx.cancel.is_cancelled() {
                anyhow::bail!(ArchiveError::Cancelled);
            }
            let header = header?;
            let name = header.filename.to_string_lossy().to_string();
            // 校验路径；非法路径直接拒绝整个归档，避免 unrar 写入逃逸文件。
            sanitize_entry_path(&name, ctx.dest)?;
            // 单文件大小上限校验，防解压炸弹。
            ctx.options.limits.check_file_size(header.unpacked_size)?;
            entries.push(PreflightEntry {
                name,
                is_directory: header.is_directory(),
                unpacked_size: header.unpacked_size,
            });
        }
        // 条目数上限校验，防海量条目耗尽资源。
        ctx.options.limits.check_entries(entries.len())?;
        Ok(Preflight {
            count: entries.len(),
            entries,
        })
    }
}

struct PreflightEntry {
    name: String,
    is_directory: bool,
    unpacked_size: u64,
}

struct Preflight {
    count: usize,
    entries: Vec<PreflightEntry>,
}

struct IsolationGuard {
    path: PathBuf,
    keep: bool,
}

impl IsolationGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, keep: false }
    }

    fn commit(&mut self) {
        self.keep = true;
    }

    fn remove_now(&self) -> std::io::Result<()> {
        if self.path.exists() {
            fs::remove_dir_all(&self.path)
        } else {
            Ok(())
        }
    }
}

impl Drop for IsolationGuard {
    fn drop(&mut self) {
        if !self.keep {
            let _ = self.remove_now();
        }
    }
}

fn unique_isolation_dir(dest: &Path) -> anyhow::Result<PathBuf> {
    let parent = dest.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let canon_parent = parent.canonicalize()?;
    let dest_name = dest
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("extractr");
    for _ in 0..128 {
        let sequence = ISOLATION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let candidate = canon_parent.join(format!(
            ".{dest_name}.isolate-{}-{sequence}",
            std::process::id()
        ));
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    anyhow::bail!("无法创建隔离解压目录")
}

/// 把隔离目录中的每个条目安全提交到目标目录。
fn commit_isolation(
    ctx: &ExtractContext,
    isolation: &Path,
    preflight: &Preflight,
    cancel: &dyn crate::traits::CancelToken,
) -> anyhow::Result<ExtractSummary> {
    let total_bytes = preflight
        .entries
        .iter()
        .filter(|entry| !entry.is_directory)
        .map(|entry| entry.unpacked_size)
        .sum::<u64>();
    let mut summary = ExtractSummary::default();
    let mut bytes_done = 0_u64;
    for (index, entry) in preflight.entries.iter().enumerate() {
        if cancel.is_cancelled() {
            summary.cancelled = true;
            break;
        }
        let relative = sanitize_entry_path(&entry.name, ctx.dest)?;
        let metadata = ArchiveEntry {
            path: entry.name.clone(),
            size: entry.unpacked_size,
            compressed_size: 0,
            is_dir: entry.is_directory,
            is_encrypted: ctx.options.password.is_some(),
            modified: None,
        };
        ctx.progress
            .on_entry_start(index, preflight.count, &metadata);

        // 单文件大小上限校验，防解压炸弹。
        ctx.options.limits.check_file_size(entry.unpacked_size)?;
        // 累计总字节上限校验。
        if bytes_done + entry.unpacked_size > ctx.options.limits.max_total_bytes {
            return Err(crate::error::ArchiveError::BombDetected {
                current: bytes_done + entry.unpacked_size,
                max: ctx.options.limits.max_total_bytes,
            }
            .into());
        }

        let source_in_isolation = isolation.join(&relative);

        if entry.is_directory {
            let target = ctx.dest.join(&relative);
            ensure_safe_directory(ctx.dest, &relative)?;
            copy_dir_recursive(&source_in_isolation, &target)?;
            summary.entries_extracted += 1;
            ctx.progress.on_entry_done(index, 0);
            continue;
        }

        let target = ctx.dest.join(&relative);
        let Some(target) = resolve_target_path(&target, ctx.options.overwrite)? else {
            summary.entries_skipped += 1;
            bytes_done += entry.unpacked_size;
            ctx.progress.on_progress(bytes_done, total_bytes);
            ctx.progress.on_entry_done(index, 0);
            continue;
        };

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        // 同卷原子移动。若目标已存在且策略要求覆盖，Windows 不允许跨文件 rename
        // 覆盖，这里用“先备份旧文件再移动”的安全提交，失败时恢复旧文件。
        move_safely(&source_in_isolation, &target)?;
        summary.bytes_written += entry.unpacked_size;
        summary.entries_extracted += 1;
        bytes_done += entry.unpacked_size;
        ctx.progress.on_progress(bytes_done, total_bytes);
        ctx.progress.on_entry_done(index, entry.unpacked_size);
    }
    Ok(summary)
}

fn move_safely(source: &Path, target: &Path) -> anyhow::Result<()> {
    if !source.exists() {
        anyhow::bail!("隔离目录缺少条目: {}", source.display());
    }
    let backup = if target.exists() {
        Some(unique_sibling_path(target, "backup")?)
    } else {
        None
    };
    if let Some(backup) = &backup {
        fs::rename(target, backup)?;
    }
    if let Err(error) = fs::rename(source, target) {
        if let Some(backup) = &backup {
            let _ = fs::rename(backup, target);
        }
        return Err(error.into());
    }
    if let Some(backup) = backup {
        fs::remove_file(backup)?;
    }
    Ok(())
}

fn copy_dir_recursive(source: &Path, target: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path)?;
        }
    }
    Ok(())
}

fn unique_sibling_path(target: &Path, label: &str) -> anyhow::Result<PathBuf> {
    let parent = target
        .parent()
        .ok_or_else(|| anyhow::anyhow!("目标文件没有父目录"))?;
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("无法解析目标文件名: {}", target.display()))?;
    let sequence = ISOLATION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    Ok(parent.join(format!(
        ".{name}.extractr-{}-{sequence}.{label}",
        std::process::id()
    )))
}

fn looks_like_password_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("password") || lower.contains("encrypt")
}
