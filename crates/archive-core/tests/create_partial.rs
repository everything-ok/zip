//! 创建归档与部分解压的集成测试（Batch 9/10 新能力）。

use std::io::Write;

use archive_core::creators;
use archive_core::dispatcher;
use archive_core::progress::{AtomicCancel, NoopSink};
use archive_core::traits::{CreateContext, CreateOptions, CreateSource, ExtractContext};
use archive_core::{ExtractOptions, OverwritePolicy};

#[test]
fn create_zip_then_extract_roundtrip() {
    let tmp = tempfile::tempdir().expect("tempdir");
    // 准备源文件。
    let src_dir = tmp.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("a.txt"), b"alpha").unwrap();
    std::fs::write(src_dir.join("b.txt"), b"beta data").unwrap();

    let archive_path = tmp.path().join("out.zip");
    let sources = vec![
        CreateSource {
            fs_path: src_dir.join("a.txt"),
            archive_path: "a.txt".into(),
        },
        CreateSource {
            fs_path: src_dir.join("b.txt"),
            archive_path: "sub/b.txt".into(),
        },
    ];
    let options = CreateOptions { password: None, level: Some(6) };
    let cancel = AtomicCancel::new();
    let creator = creators::creator_for_path(&archive_path).expect("creator");
    let summary = creator
        .create(&CreateContext {
            dest: &archive_path,
            sources: &sources,
            options: &options,
            progress: &NoopSink,
            cancel: &cancel,
        })
        .expect("create");
    assert_eq!(summary.entries_extracted, 2);
    assert!(archive_path.exists());

    // 解压回来验证内容一致。
    let dest = tmp.path().join("extracted");
    let extractor = dispatcher::open(&archive_path).expect("open");
    let opts = ExtractOptions::default();
    let summary = extractor
        .extract(&ExtractContext {
            source: &archive_path,
            dest: &dest,
            options: &opts,
            progress: &NoopSink,
            cancel: &cancel,
        })
        .expect("extract");
    assert_eq!(summary.entries_extracted, 2);
    assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"alpha");
    assert_eq!(std::fs::read(dest.join("sub/b.txt")).unwrap(), b"beta data");
}

#[test]
fn partial_extract_only_selected_entries() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let archive_path = tmp.path().join("partial.zip");
    let mut writer = zip::ZipWriter::new(std::fs::File::create(&archive_path).expect("create zip"));
    let opts = zip::write::SimpleFileOptions::default();
    for (name, data) in [("keep.txt", b"keep"), ("skip.txt", b"skip"), ("dir/x.txt", b"deep")] {
        writer.start_file(name, opts).expect("start");
        writer.write_all(data).expect("write");
    }
    writer.finish().expect("finish");

    let dest = tmp.path().join("out");
    let extractor = dispatcher::open(&archive_path).expect("open zip");
    assert!(extractor.supports_partial());
    let opts = ExtractOptions { overwrite: OverwritePolicy::Overwrite, ..Default::default() };
    let cancel = AtomicCancel::new();
    let summary = extractor
        .extract_entries(
            &ExtractContext {
                source: &archive_path,
                dest: &dest,
                options: &opts,
                progress: &NoopSink,
                cancel: &cancel,
            },
            &["keep.txt".to_string(), "dir/x.txt".to_string()],
        )
        .expect("partial extract");
    // 只解压选中的两个，skip.txt 不应存在。
    assert!(dest.join("keep.txt").exists());
    assert!(dest.join("dir/x.txt").exists());
    assert!(!dest.join("skip.txt").exists(), "未选中的条目不应被解压");
    assert_eq!(summary.entries_extracted, 2);
}

#[test]
fn partial_extract_directory_prefix() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let archive_path = tmp.path().join("dirprefix.zip");
    let mut writer = zip::ZipWriter::new(std::fs::File::create(&archive_path).expect("create zip"));
    let opts = zip::write::SimpleFileOptions::default();
    writer.start_file("dir/a.txt", opts).expect("start");
    writer.write_all(b"a").expect("write");
    writer.start_file("dir/b.txt", opts).expect("start");
    writer.write_all(b"b").expect("write");
    writer.start_file("other/c.txt", opts).expect("start");
    writer.write_all(b"c").expect("write");
    writer.finish().expect("finish");

    let dest = tmp.path().join("out");
    let extractor = dispatcher::open(&archive_path).expect("open zip");
    let opts = ExtractOptions::default();
    let cancel = AtomicCancel::new();
    let _ = extractor
        .extract_entries(
            &ExtractContext {
                source: &archive_path,
                dest: &dest,
                options: &opts,
                progress: &NoopSink,
                cancel: &cancel,
            },
            &["dir/".to_string()],
        )
        .expect("partial extract");
    // 目录前缀 dir/ 下的文件都应解压，other/ 不应解压。
    assert!(dest.join("dir/a.txt").exists());
    assert!(dest.join("dir/b.txt").exists());
    assert!(!dest.join("other/c.txt").exists());
}

#[test]
fn tar_does_not_support_partial() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let archive_path = tmp.path().join("x.tar");
    let mut builder = tar::Builder::new(std::fs::File::create(&archive_path).expect("tar"));
    let mut header = tar::Header::new_gnu();
    header.set_path("a.txt").unwrap();
    header.set_size(3);
    header.set_mode(0o644);
    header.set_cksum();
    builder.append(&header, std::io::Cursor::new(b"abc")).expect("append");
    builder.finish().expect("finish");

    let dest = tmp.path().join("out");
    let extractor = dispatcher::open(&archive_path).expect("open tar");
    assert!(!extractor.supports_partial(), "TAR 流式格式不支持部分解压");
    let opts = ExtractOptions::default();
    let cancel = AtomicCancel::new();
    let result = extractor.extract_entries(
        &ExtractContext {
            source: &archive_path,
            dest: &dest,
            options: &opts,
            progress: &NoopSink,
            cancel: &cancel,
        },
        &["a.txt".to_string()],
    );
    assert!(result.is_err(), "TAR 部分解压应返回 unsupported");
}

#[test]
fn create_zip_cancellation() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src = tmp.path().join("big.txt");
    std::fs::write(&src, &vec![0x41; 512 * 1024]).unwrap();
    let archive_path = tmp.path().join("cancel.zip");
    let sources = vec![CreateSource {
        fs_path: src,
        archive_path: "big.txt".into(),
    }];
    let options = CreateOptions::default();

    // 立即取消的令牌。
    struct CancelNow;
    impl archive_core::traits::CancelToken for CancelNow {
        fn is_cancelled(&self) -> bool {
            true
        }
    }
    let creator = creators::creator_for_path(&archive_path).expect("creator");
    let summary = creator
        .create(&CreateContext {
            dest: &archive_path,
            sources: &sources,
            options: &options,
            progress: &NoopSink,
            cancel: &CancelNow,
        })
        .expect("create should not error on cancel");
    assert!(summary.cancelled, "取消应反映在 summary");
}

#[test]
fn unsupported_create_format_is_rejected() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let result = creators::creator_for_path(&tmp.path().join("x.rar"));
    assert!(result.is_err(), "RAR 创建应被拒绝（当前仅支持 zip）");
}
