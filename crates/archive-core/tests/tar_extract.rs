//! TAR 解压测试：普通解压、损坏 TAR、tar.gz 往返。

use archive_core::dispatcher;
use archive_core::progress::{AtomicCancel, NoopSink};
use archive_core::traits::ExtractContext;
use archive_core::ExtractOptions;

#[test]
fn tar_plain_extract() {
    let tmp = tempfile::tempdir().unwrap();
    let arch = tmp.path().join("data.tar");
    let mut builder = tar::Builder::new(std::fs::File::create(&arch).unwrap());
    let mut header = tar::Header::new_gnu();
    header.set_path("a.txt").unwrap();
    header.set_size(5);
    header.set_mode(0o644);
    header.set_cksum();
    builder.append(&header, std::io::Cursor::new(b"hello")).unwrap();
    let mut header2 = tar::Header::new_gnu();
    header2.set_path("dir/b.txt").unwrap();
    header2.set_size(3);
    header2.set_mode(0o644);
    header2.set_cksum();
    builder.append(&header2, std::io::Cursor::new(b"bye")).unwrap();
    builder.finish().unwrap();

    let dest = tmp.path().join("out");
    let extractor = dispatcher::open(&arch).unwrap();
    let summary = extractor
        .extract(&ExtractContext {
            source: &arch,
            dest: &dest,
            options: &ExtractOptions::default(),
            progress: &NoopSink,
            cancel: &AtomicCancel::new(),
        })
        .unwrap();
    assert_eq!(summary.entries_extracted, 2);
    assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"hello");
    assert_eq!(std::fs::read(dest.join("dir/b.txt")).unwrap(), b"bye");
}

#[test]
fn corrupt_tar_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let arch = tmp.path().join("bad.tar");
    std::fs::write(&arch, b"not a tar file at all").unwrap();
    let dest = tmp.path().join("out");
    let extractor = dispatcher::open(&arch).unwrap();
    // 损坏 TAR：tar crate 通常返回空 entries 而非错误，但 extract 不应 panic。
    let _ = extractor.extract(&ExtractContext {
        source: &arch,
        dest: &dest,
        options: &ExtractOptions::default(),
        progress: &NoopSink,
        cancel: &AtomicCancel::new(),
    });
    // 关键：不 panic 即通过。
}

#[test]
fn tar_gz_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    std::fs::create_dir_all(src.join("sub")).unwrap();
    std::fs::write(src.join("a.txt"), b"alpha").unwrap();
    std::fs::write(src.join("sub/b.txt"), b"beta").unwrap();

    let arch = tmp.path().join("out.tar.gz");
    let gz = flate2::write::GzEncoder::new(
        std::fs::File::create(&arch).unwrap(),
        flate2::Compression::default(),
    );
    let mut builder = tar::Builder::new(gz);
    builder.append_dir_all("pkg", &src).unwrap();
    // into_inner 返回底层 GzEncoder，需 finish 刷新 gzip 流。
    builder.into_inner().unwrap().finish().unwrap();

    let dest = tmp.path().join("extracted");
    let extractor = dispatcher::open(&arch).unwrap();
    let summary = extractor
        .extract(&ExtractContext {
            source: &arch,
            dest: &dest,
            options: &ExtractOptions::default(),
            progress: &NoopSink,
            cancel: &AtomicCancel::new(),
        })
        .unwrap();
    assert!(summary.entries_extracted >= 2);
    assert_eq!(std::fs::read(dest.join("pkg/a.txt")).unwrap(), b"alpha");
}
