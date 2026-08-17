//! gz/bz2/xz/zst 单文件压缩流解压测试 + strip_compound_ext 边界。

use std::io::Write;

use archive_core::dispatcher;
use archive_core::progress::{AtomicCancel, NoopSink};
use archive_core::traits::ExtractContext;
use archive_core::ExtractOptions;

fn run_gz(archive_path: &std::path::Path, dest: &std::path::Path) -> anyhow::Result<archive_core::ExtractSummary> {
    let extractor = dispatcher::open(archive_path)?;
    let opts = ExtractOptions::default();
    let cancel = AtomicCancel::new();
    extractor.extract(&ExtractContext {
        source: archive_path,
        dest,
        options: &opts,
        progress: &NoopSink,
        cancel: &cancel,
    })
}

#[test]
fn gzip_single_file_extract() {
    let tmp = tempfile::tempdir().unwrap();
    let raw = tmp.path().join("data.txt");
    std::fs::write(&raw, b"hello gzip world").unwrap();
    // 归档名 data.txt.gz → strip_compound_ext 去掉 .gz 得 "data.txt"。
    let arch = tmp.path().join("data.txt.gz");
    let mut enc = flate2::write::GzEncoder::new(std::fs::File::create(&arch).unwrap(), flate2::Compression::default());
    enc.write_all(b"hello gzip world").unwrap();
    enc.finish().unwrap();
    std::fs::remove_file(&raw).unwrap();

    let dest = tmp.path().join("out");
    let summary = run_gz(&arch, &dest).unwrap();
    assert_eq!(summary.entries_extracted, 1);
    assert_eq!(std::fs::read(dest.join("data.txt")).unwrap(), b"hello gzip world");
}

#[test]
fn xz_single_file_extract() {
    let tmp = tempfile::tempdir().unwrap();
    let raw = tmp.path().join("data.bin");
    std::fs::write(&raw, vec![0xABu8; 8192]).unwrap();
    let arch = tmp.path().join("data.bin.xz");
    let mut enc = xz2::write::XzEncoder::new(std::fs::File::create(&arch).unwrap(), 6);
    enc.write_all(&vec![0xABu8; 8192]).unwrap();
    enc.finish().unwrap();
    std::fs::remove_file(&raw).unwrap();

    let dest = tmp.path().join("out");
    let summary = run_gz(&arch, &dest).unwrap();
    assert_eq!(summary.entries_extracted, 1);
    assert_eq!(std::fs::read(dest.join("data.bin")).unwrap().len(), 8192);
}

#[test]
fn zstd_single_file_extract() {
    let tmp = tempfile::tempdir().unwrap();
    let raw = tmp.path().join("data.txt");
    std::fs::write(&raw, b"zstandard content").unwrap();
    let arch = tmp.path().join("data.txt.zst");
    let mut enc = zstd::stream::Encoder::new(std::fs::File::create(&arch).unwrap(), 3).unwrap();
    enc.write_all(b"zstandard content").unwrap();
    enc.finish().unwrap();
    std::fs::remove_file(&raw).unwrap();

    let dest = tmp.path().join("out");
    let summary = run_gz(&arch, &dest).unwrap();
    assert_eq!(summary.entries_extracted, 1);
    assert_eq!(std::fs::read(dest.join("data.txt")).unwrap(), b"zstandard content");
}

#[test]
fn corrupt_gzip_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let arch = tmp.path().join("bad.gz");
    // 非 gzip magic。
    std::fs::write(&arch, b"not a gzip file at all").unwrap();
    let dest = tmp.path().join("out");
    let result = run_gz(&arch, &dest);
    assert!(result.is_err(), "损坏 gzip 应解压失败");
}
