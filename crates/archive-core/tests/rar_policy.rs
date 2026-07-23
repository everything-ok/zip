//! RAR 隔离解压与覆盖策略测试。
//!
//! 环境中通常没有 RARLab 的 `rar` 工具来生成真实 RAR 归档，因此这些测试采用
//! “如果存在样本则验证策略，否则跳过”的策略。`RAR_SAMPLE` 环境变量指向一个
//! 未加密、含 `data.txt` 条目的 RAR 样本时，测试会覆盖各覆盖策略。

use std::env;
use std::path::PathBuf;

use archive_core::dispatcher;
use archive_core::progress::{AtomicCancel, NoopSink};
use archive_core::traits::{CancelToken, ExtractContext};
use archive_core::{ExtractOptions, OverwritePolicy};

/// 取环境变量 `RAR_SAMPLE` 指向的 RAR 样本路径；不存在时返回 None 以跳过测试。
fn sample() -> Option<PathBuf> {
    let path = PathBuf::from(env::var("RAR_SAMPLE").ok()?);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

fn run(
    archive_path: &std::path::Path,
    dest: &std::path::Path,
    overwrite: OverwritePolicy,
    cancel: &dyn CancelToken,
) -> anyhow::Result<archive_core::ExtractSummary> {
    let extractor = dispatcher::open(archive_path).expect("open rar");
    let options = ExtractOptions {
        overwrite,
        ..Default::default()
    };
    let sink = NoopSink;
    extractor.extract(&ExtractContext {
        source: archive_path,
        dest,
        options: &options,
        progress: &sink,
        cancel,
    })
}

struct CancelImmediately;
impl CancelToken for CancelImmediately {
    fn is_cancelled(&self) -> bool {
        true
    }
}

#[test]
fn extracts_new_archive() {
    let Some(archive) = sample() else {
        eprintln!("跳过：未设置 RAR_SAMPLE");
        return;
    };
    let temp = tempfile::tempdir().expect("tempdir");
    let dest = temp.path().join("out");
    let cancel = AtomicCancel::new();
    let summary = run(&archive, &dest, OverwritePolicy::Skip, &cancel).expect("extract");
    assert!(summary.entries_extracted >= 1);
    assert!(dest.join("data.txt").exists());
    let leftover = std::fs::read_dir(&dest)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().contains(".isolate-"))
        .count();
    assert_eq!(leftover, 0, "隔离目录不应残留");
}

#[test]
fn overwrite_policy_replaces_existing() {
    let Some(archive) = sample() else {
        eprintln!("跳过：未设置 RAR_SAMPLE");
        return;
    };
    let temp = tempfile::tempdir().expect("tempdir");
    let dest = temp.path().join("out");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("data.txt"), b"old").unwrap();
    let cancel = AtomicCancel::new();
    let summary = run(&archive, &dest, OverwritePolicy::Overwrite, &cancel).expect("extract");
    assert!(summary.entries_extracted >= 1);
    let content = std::fs::read(dest.join("data.txt")).unwrap();
    assert_ne!(content, b"old", "覆盖策略应替换旧文件");
}

#[test]
fn skip_policy_keeps_existing_file() {
    let Some(archive) = sample() else {
        eprintln!("跳过：未设置 RAR_SAMPLE");
        return;
    };
    let temp = tempfile::tempdir().expect("tempdir");
    let dest = temp.path().join("out");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("data.txt"), b"old").unwrap();
    let cancel = AtomicCancel::new();
    let summary = run(&archive, &dest, OverwritePolicy::Skip, &cancel).expect("extract");
    assert!(summary.entries_skipped >= 1);
    assert_eq!(std::fs::read(dest.join("data.txt")).unwrap(), b"old");
}

#[test]
fn rename_policy_keeps_both() {
    let Some(archive) = sample() else {
        eprintln!("跳过：未设置 RAR_SAMPLE");
        return;
    };
    let temp = tempfile::tempdir().expect("tempdir");
    let dest = temp.path().join("out");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("data.txt"), b"old").unwrap();
    let cancel = AtomicCancel::new();
    let summary = run(&archive, &dest, OverwritePolicy::Rename, &cancel).expect("extract");
    assert!(summary.entries_extracted >= 1);
    assert_eq!(std::fs::read(dest.join("data.txt")).unwrap(), b"old");
    assert!(dest.join("data (1).txt").exists(), "应生成重命名副本");
}

#[test]
fn cancellation_does_not_damage_existing() {
    let Some(archive) = sample() else {
        eprintln!("跳过：未设置 RAR_SAMPLE");
        return;
    };
    let temp = tempfile::tempdir().expect("tempdir");
    let dest = temp.path().join("out");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("data.txt"), b"keep").unwrap();
    let summary = run(
        &archive,
        &dest,
        OverwritePolicy::Overwrite,
        &CancelImmediately,
    )
    .expect("cancel outcome");
    assert!(summary.cancelled);
    assert_eq!(std::fs::read(dest.join("data.txt")).unwrap(), b"keep");
}
