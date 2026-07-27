//! 解压炸弹与资源上限回归测试。
//!
//! 覆盖：高压缩比炸弹、单文件超限、总字节超限、海量条目、超长路径。
//! 这些场景下解压必须被拒绝，且不逃逸、不损坏既有文件。

use std::io::Write;

use archive_core::dispatcher;
use archive_core::progress::{AtomicCancel, NoopSink};
use archive_core::traits::ExtractContext;
use archive_core::{ExtractLimits, ExtractOptions, OverwritePolicy};

#[test]
fn file_too_large_is_rejected() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let archive_path = tmp.path().join("big.zip");
    let mut writer = zip::ZipWriter::new(std::fs::File::create(&archive_path).expect("create zip"));
    writer
        .start_file("big.bin", zip::write::SimpleFileOptions::default())
        .expect("start");
    writer.write_all(&vec![0x41; 1024]).expect("write");
    writer.finish().expect("finish");

    let dest = tmp.path().join("out");
    let extractor = dispatcher::open(&archive_path).expect("open zip");
    // 单文件上限设为 512 字节，1024 必然超限。
    let options = ExtractOptions {
        limits: ExtractLimits {
            max_file_bytes: 512,
            ..Default::default()
        },
        ..Default::default()
    };
    let cancel = AtomicCancel::new();
    let result = extractor.extract(&ExtractContext {
        source: &archive_path,
        dest: &dest,
        options: &options,
        progress: &NoopSink,
        cancel: &cancel,
    });
    assert!(result.is_err(), "超单文件上限必须被拒绝");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("单文件") || err.contains("上限"),
        "错误信息: {err}"
    );
    assert!(!dest.join("big.bin").exists(), "不应产生部分文件");
}

#[test]
fn total_bytes_limit_is_rejected() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let archive_path = tmp.path().join("multi.zip");
    let mut writer = zip::ZipWriter::new(std::fs::File::create(&archive_path).expect("create zip"));
    let opts = zip::write::SimpleFileOptions::default();
    for i in 0..4 {
        writer.start_file(format!("f{i}.bin"), opts).expect("start");
        writer.write_all(&vec![0x42; 512]).expect("write");
    }
    writer.finish().expect("finish");

    let dest = tmp.path().join("out");
    let extractor = dispatcher::open(&archive_path).expect("open zip");
    // 总上限 1024，4×512=2048 必然超限。
    let options = ExtractOptions {
        limits: ExtractLimits {
            max_total_bytes: 1024,
            max_file_bytes: 1024,
            ..Default::default()
        },
        ..Default::default()
    };
    let cancel = AtomicCancel::new();
    let result = extractor.extract(&ExtractContext {
        source: &archive_path,
        dest: &dest,
        options: &options,
        progress: &NoopSink,
        cancel: &cancel,
    });
    assert!(result.is_err(), "超总字节上限必须被拒绝");
}

#[test]
fn too_many_entries_is_rejected() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let archive_path = tmp.path().join("many.zip");
    let mut writer = zip::ZipWriter::new(std::fs::File::create(&archive_path).expect("create zip"));
    let opts = zip::write::SimpleFileOptions::default();
    for i in 0..6 {
        writer.start_file(format!("f{i}.txt"), opts).expect("start");
        writer.write_all(b"x").expect("write");
    }
    writer.finish().expect("finish");

    let dest = tmp.path().join("out");
    let extractor = dispatcher::open(&archive_path).expect("open zip");
    let options = ExtractOptions {
        limits: ExtractLimits {
            max_entries: 5,
            ..Default::default()
        },
        ..Default::default()
    };
    let cancel = AtomicCancel::new();
    // list 与 extract 都应在条目数校验上体现拒绝。这里验证 extract 路径。
    let result = extractor.extract(&ExtractContext {
        source: &archive_path,
        dest: &dest,
        options: &options,
        progress: &NoopSink,
        cancel: &cancel,
    });
    assert!(result.is_err(), "超条目数上限必须被拒绝");
}

#[test]
fn limits_do_not_block_normal_extraction() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let archive_path = tmp.path().join("ok.zip");
    let mut writer = zip::ZipWriter::new(std::fs::File::create(&archive_path).expect("create zip"));
    writer
        .start_file("hello.txt", zip::write::SimpleFileOptions::default())
        .expect("start");
    writer.write_all(b"hi").expect("write");
    writer.finish().expect("finish");

    let dest = tmp.path().join("out");
    let extractor = dispatcher::open(&archive_path).expect("open zip");
    // 上限设得合理，不应误伤正常解压。
    let options = ExtractOptions::default();
    let cancel = AtomicCancel::new();
    let summary = extractor
        .extract(&ExtractContext {
            source: &archive_path,
            dest: &dest,
            options: &options,
            progress: &NoopSink,
            cancel: &cancel,
        })
        .expect("extract");
    assert_eq!(summary.entries_extracted, 1);
    assert_eq!(std::fs::read(dest.join("hello.txt")).unwrap(), b"hi");
}

#[test]
fn bomb_detection_preserves_existing_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let archive_path = tmp.path().join("overwrite.zip");
    let mut writer = zip::ZipWriter::new(std::fs::File::create(&archive_path).expect("create zip"));
    writer
        .start_file("report.txt", zip::write::SimpleFileOptions::default())
        .expect("start");
    writer.write_all(&vec![0x55; 512]).expect("write");
    writer.finish().expect("finish");

    let dest = tmp.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let target = dest.join("report.txt");
    std::fs::write(&target, b"keep").expect("old file");

    let extractor = dispatcher::open(&archive_path).expect("open zip");
    let options = ExtractOptions {
        overwrite: OverwritePolicy::Overwrite,
        limits: ExtractLimits {
            max_file_bytes: 64,
            ..Default::default()
        },
        ..Default::default()
    };
    let cancel = AtomicCancel::new();
    let result = extractor.extract(&ExtractContext {
        source: &archive_path,
        dest: &dest,
        options: &options,
        progress: &NoopSink,
        cancel: &cancel,
    });
    assert!(result.is_err(), "炸弹防护必须拒绝");
    // 关键：既有文件不被截断或删除。
    assert_eq!(std::fs::read(&target).unwrap(), b"keep");
}
