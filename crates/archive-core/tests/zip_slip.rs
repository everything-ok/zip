//! Zip Slip、原子输出与 TAR 条目安全回归测试。

use std::io::Write;

use archive_core::dispatcher;
use archive_core::progress::{AtomicCancel, NoopSink};
use archive_core::safety;
use archive_core::traits::{CancelToken, ExtractContext};
use archive_core::{ExtractOptions, OverwritePolicy};

#[test]
fn sanitize_rejects_traversal() {
    let dest = std::env::temp_dir();
    assert!(safety::sanitize_entry_path("../evil.exe", &dest).is_err());
    assert!(safety::sanitize_entry_path("..\\..\\evil", &dest).is_err());
    assert!(safety::sanitize_entry_path("dir/../../evil", &dest).is_err());
}

#[test]
fn sanitize_rejects_windows_drive_and_reserved() {
    let dest = std::env::temp_dir();
    assert!(safety::sanitize_entry_path("C:/Windows/x", &dest).is_err());
    assert!(safety::sanitize_entry_path("CON.txt", &dest).is_err());
    assert!(safety::sanitize_entry_path("dir/NUL", &dest).is_err());
    assert!(safety::sanitize_entry_path("file:bad", &dest).is_err());
}

#[test]
fn sanitize_is_side_effect_free() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let destination = tmp.path().join("does-not-exist");
    let safe = safety::sanitize_entry_path("deep/path/file.txt", &destination).expect("safe path");
    assert_eq!(safe, std::path::PathBuf::from("deep/path/file.txt"));
    assert!(!destination.exists(), "路径验证不得创建输出目录");
}

#[test]
fn zip_slip_does_not_escape() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let archive_path = tmp.path().join("evil.zip");
    let file = std::fs::File::create(&archive_path).expect("create zip");
    let mut writer = zip::ZipWriter::new(file);
    writer
        .start_file("../escaped.txt", zip::write::SimpleFileOptions::default())
        .expect("start malicious entry");
    writer.write_all(b"evil").expect("write");
    writer.finish().expect("finish zip");

    let destination = tmp.path().join("out");
    let extractor = dispatcher::open(&archive_path).expect("open");
    let options = ExtractOptions::default();
    let sink = NoopSink;
    let cancel = AtomicCancel::new();
    let result = extractor.extract(&ExtractContext {
        source: &archive_path,
        dest: &destination,
        options: &options,
        progress: &sink,
        cancel: &cancel,
    });
    assert!(result.is_err(), "恶意归档必须被拒绝");
    assert!(!tmp.path().join("escaped.txt").exists(), "Zip Slip 逃逸");
}

#[test]
fn extracts_normal_zip() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let archive_path = tmp.path().join("normal.zip");
    let file = std::fs::File::create(&archive_path).expect("create zip");
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    writer.start_file("hello.txt", options).expect("start");
    writer.write_all(b"hi").expect("write");
    writer
        .start_file("sub/deep/world.txt", options)
        .expect("start2");
    writer.write_all(b"deep").expect("write2");
    writer.finish().expect("finish");

    let destination = tmp.path().join("out");
    let extractor = dispatcher::open(&archive_path).expect("open");
    let options = ExtractOptions::default();
    let sink = NoopSink;
    let cancel = AtomicCancel::new();
    let summary = extractor
        .extract(&ExtractContext {
            source: &archive_path,
            dest: &destination,
            options: &options,
            progress: &sink,
            cancel: &cancel,
        })
        .expect("extract");
    assert_eq!(summary.entries_extracted, 2);
    assert_eq!(std::fs::read(destination.join("hello.txt")).unwrap(), b"hi");
    assert_eq!(
        std::fs::read(destination.join("sub/deep/world.txt")).unwrap(),
        b"deep"
    );
}

struct CancelAfterFirstChunk {
    cancelled: std::sync::atomic::AtomicBool,
}

impl CancelAfterFirstChunk {
    fn new() -> Self {
        Self {
            cancelled: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

impl CancelToken for CancelAfterFirstChunk {
    fn is_cancelled(&self) -> bool {
        self.cancelled
            .swap(true, std::sync::atomic::Ordering::SeqCst)
    }
}

#[test]
fn cancellation_does_not_damage_overwritten_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let archive_path = tmp.path().join("replace.zip");
    let file = std::fs::File::create(&archive_path).expect("create zip");
    let mut writer = zip::ZipWriter::new(file);
    writer
        .start_file("report.txt", zip::write::SimpleFileOptions::default())
        .expect("start");
    writer.write_all(&vec![0x55; 512 * 1024]).expect("write");
    writer.finish().expect("finish");

    let destination = tmp.path().join("out");
    std::fs::create_dir_all(&destination).expect("destination");
    let target = destination.join("report.txt");
    std::fs::write(&target, b"keep the old content").expect("old file");

    let extractor = dispatcher::open(&archive_path).expect("open");
    let options = ExtractOptions {
        overwrite: OverwritePolicy::Overwrite,
        ..Default::default()
    };
    let sink = NoopSink;
    let cancel = CancelAfterFirstChunk::new();
    let summary = extractor
        .extract(&ExtractContext {
            source: &archive_path,
            dest: &destination,
            options: &options,
            progress: &sink,
            cancel: &cancel,
        })
        .expect("cancel is a normal outcome");
    assert!(summary.cancelled);
    assert_eq!(std::fs::read(&target).unwrap(), b"keep the old content");
}

#[test]
fn tar_special_entries_are_skipped() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let archive_path = tmp.path().join("special.tar");
    let mut builder = tar::Builder::new(std::fs::File::create(&archive_path).expect("tar"));
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::fifo());
    header.set_size(0);
    header.set_cksum();
    builder
        .append_data(&mut header, "pipe", std::io::empty())
        .expect("append fifo");
    builder.finish().expect("finish tar");

    let destination = tmp.path().join("out");
    let extractor = dispatcher::open(&archive_path).expect("open tar");
    let options = ExtractOptions::default();
    let sink = NoopSink;
    let cancel = AtomicCancel::new();
    let summary = extractor
        .extract(&ExtractContext {
            source: &archive_path,
            dest: &destination,
            options: &options,
            progress: &sink,
            cancel: &cancel,
        })
        .expect("extract tar");
    assert_eq!(summary.entries_skipped, 1);
    assert!(!destination.join("pipe").exists());
}
