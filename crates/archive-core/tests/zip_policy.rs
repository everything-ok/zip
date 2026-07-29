//! ZIP 覆盖策略矩阵测试（Skip/Overwrite/Rename/Error + 取消）。

use std::io::Write;

use archive_core::dispatcher;
use archive_core::progress::{AtomicCancel, NoopSink};
use archive_core::traits::{CancelToken, ExtractContext};
use archive_core::{ExtractOptions, OverwritePolicy};

struct CancelImmediately;
impl CancelToken for CancelImmediately {
    fn is_cancelled(&self) -> bool {
        true
    }
}

fn make_zip(entry: &str, data: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("a.zip");
    let mut w = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    w.start_file(entry, zip::write::SimpleFileOptions::default()).unwrap();
    w.write_all(data).unwrap();
    w.finish().unwrap();
    (tmp, path)
}

fn run(
    arch: &std::path::Path,
    dest: &std::path::Path,
    overwrite: OverwritePolicy,
    cancel: &dyn CancelToken,
) -> anyhow::Result<archive_core::ExtractSummary> {
    let extractor = dispatcher::open(arch)?;
    extractor.extract(&ExtractContext {
        source: arch,
        dest,
        options: &ExtractOptions { overwrite, ..Default::default() },
        progress: &NoopSink,
        cancel,
    })
}

#[test]
fn skip_keeps_existing() {
    let (tmp, arch) = make_zip("a.txt", b"new");
    let dest = tmp.path().join("out");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("a.txt"), b"old").unwrap();
    let summary = run(&arch, &dest, OverwritePolicy::Skip, &AtomicCancel::new()).unwrap();
    assert_eq!(summary.entries_skipped, 1);
    assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"old");
}

#[test]
fn overwrite_replaces_existing() {
    let (tmp, arch) = make_zip("a.txt", b"new");
    let dest = tmp.path().join("out");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("a.txt"), b"old").unwrap();
    let summary = run(&arch, &dest, OverwritePolicy::Overwrite, &AtomicCancel::new()).unwrap();
    assert_eq!(summary.entries_extracted, 1);
    assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"new");
}

#[test]
fn rename_creates_unique() {
    let (tmp, arch) = make_zip("a.txt", b"new");
    let dest = tmp.path().join("out");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("a.txt"), b"old").unwrap();
    let summary = run(&arch, &dest, OverwritePolicy::Rename, &AtomicCancel::new()).unwrap();
    assert_eq!(summary.entries_extracted, 1);
    assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"old");
    assert_eq!(std::fs::read(dest.join("a (1).txt")).unwrap(), b"new");
}

#[test]
fn error_reports_conflict() {
    let (tmp, arch) = make_zip("a.txt", b"new");
    let dest = tmp.path().join("out");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("a.txt"), b"old").unwrap();
    let result = run(&arch, &dest, OverwritePolicy::Error, &AtomicCancel::new());
    assert!(result.is_err());
    assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"old");
}

#[test]
fn cancel_keeps_existing() {
    let (tmp, arch) = make_zip("big.bin", &vec![0x41; 512 * 1024]);
    let dest = tmp.path().join("out");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("big.bin"), b"keep").unwrap();
    let summary = run(&arch, &dest, OverwritePolicy::Overwrite, &CancelImmediately).unwrap();
    assert!(summary.cancelled);
    assert_eq!(std::fs::read(dest.join("big.bin")).unwrap(), b"keep");
}
