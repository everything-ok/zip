//! 7z 覆盖策略、错误语义与安全输出回归测试。

use std::io::Cursor;
use std::path::PathBuf;

use archive_core::dispatcher;
use archive_core::progress::{AtomicCancel, NoopSink};
use archive_core::traits::{CancelToken, ExtractContext};
use archive_core::{ExtractOptions, OverwritePolicy};

/// 在临时目录生成一个含若干条目的未加密 7z 归档。
fn make_7z(entries: &[(&str, &[u8])]) -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().expect("tempdir");
    let archive_path = temp.path().join("sample.7z");
    let mut writer = sevenz_rust2::ArchiveWriter::create(&archive_path).expect("create writer");
    // 测试归档不加密头部，避免无密码时产生加密归档。
    writer.set_encrypt_header(false);
    for (name, data) in entries {
        writer
            .push_archive_entry(
                sevenz_rust2::ArchiveEntry::new_file(name),
                Some(Cursor::new(data.to_vec())),
            )
            .expect("push entry");
    }
    writer.finish().expect("finish 7z");
    (temp, archive_path)
}

fn make_7z_with_entry(entry_name: &str, data: &[u8]) -> (tempfile::TempDir, PathBuf) {
    make_7z(&[(entry_name, data)])
}

fn run(
    archive_path: &std::path::Path,
    dest: &std::path::Path,
    overwrite: OverwritePolicy,
    cancel: &dyn CancelToken,
) -> anyhow::Result<archive_core::ExtractSummary> {
    let extractor = dispatcher::open(archive_path).expect("open 7z");
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

#[test]
fn extracts_new_archive() {
    let (temp, archive) = make_7z(&[("a.txt", b"alpha"), ("sub/b.txt", b"beta")]);
    let dest = temp.path().join("out");
    let cancel = AtomicCancel::new();
    let summary = run(&archive, &dest, OverwritePolicy::Skip, &cancel).expect("extract");
    assert_eq!(summary.entries_extracted, 2);
    assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"alpha");
    assert_eq!(std::fs::read(dest.join("sub/b.txt")).unwrap(), b"beta");
}

#[test]
fn skip_policy_keeps_existing_file() {
    let (temp, archive) = make_7z_with_entry("a.txt", b"new");
    let dest = temp.path().join("out");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("a.txt"), b"old").unwrap();
    let cancel = AtomicCancel::new();
    let summary = run(&archive, &dest, OverwritePolicy::Skip, &cancel).expect("extract");
    assert_eq!(summary.entries_skipped, 1);
    assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"old");
}

#[test]
fn error_policy_reports_conflict_not_cancelled() {
    let (temp, archive) = make_7z_with_entry("a.txt", b"new");
    let dest = temp.path().join("out");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("a.txt"), b"old").unwrap();
    let cancel = AtomicCancel::new();
    let result = run(&archive, &dest, OverwritePolicy::Error, &cancel);
    assert!(result.is_err(), "Error 策略冲突必须返回错误而非取消");
    assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"old");
}

#[test]
fn rename_policy_creates_unique_file() {
    let (temp, archive) = make_7z_with_entry("a.txt", b"new");
    let dest = temp.path().join("out");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("a.txt"), b"old").unwrap();
    let cancel = AtomicCancel::new();
    let summary = run(&archive, &dest, OverwritePolicy::Rename, &cancel).expect("extract");
    assert_eq!(summary.entries_extracted, 1);
    assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"old");
    assert_eq!(std::fs::read(dest.join("a (1).txt")).unwrap(), b"new");
}

struct CancelImmediately;
impl CancelToken for CancelImmediately {
    fn is_cancelled(&self) -> bool {
        true
    }
}

#[test]
fn cancellation_is_reported_and_old_file_intact() {
    let (temp, archive) = make_7z_with_entry("big.bin", &vec![0x41; 256 * 1024]);
    let dest = temp.path().join("out");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("big.bin"), b"keep").unwrap();
    let summary = run(
        &archive,
        &dest,
        OverwritePolicy::Overwrite,
        &CancelImmediately,
    )
    .expect("cancel outcome");
    assert!(summary.cancelled);
    assert_eq!(std::fs::read(dest.join("big.bin")).unwrap(), b"keep");
}

#[test]
fn no_temp_files_left_after_success() {
    let (temp, archive) = make_7z_with_entry("a.txt", b"alpha");
    let dest = temp.path().join("out");
    let cancel = AtomicCancel::new();
    run(&archive, &dest, OverwritePolicy::Skip, &cancel).expect("extract");
    let leftover = std::fs::read_dir(&dest)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().contains(".extractr-"))
        .count();
    assert_eq!(leftover, 0, "成功后不应残留临时文件");
}

#[test]
fn malicious_path_is_rejected() {
    let (temp, archive) = make_7z_with_entry("../escaped.txt", b"evil");
    let dest = temp.path().join("out");
    let cancel = AtomicCancel::new();
    let result = run(&archive, &dest, OverwritePolicy::Skip, &cancel);
    assert!(result.is_err(), "恶意路径必须被拒绝");
    assert!(!temp.path().join("escaped.txt").exists(), "路径逃逸");
}
