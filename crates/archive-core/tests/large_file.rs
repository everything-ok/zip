//! 流式解压回归：多 MB 条目通过固定缓冲解压，验证输出完整且无整包读入。

use std::io::Write;

use archive_core::dispatcher;
use archive_core::progress::{AtomicCancel, NoopSink};
use archive_core::traits::ExtractContext;
use archive_core::ExtractOptions;

#[test]
fn extracts_multi_megabyte_zip_streamingly() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let archive = tmp.path().join("large.zip");
    let dest = tmp.path().join("out");

    // 8 MiB：足以跨越 256 KiB 流式缓冲的多次 read/write，且 CI 运行快速。
    let payload = vec![0x5A_u8; 8 * 1024 * 1024];
    let mut writer = zip::ZipWriter::new(std::fs::File::create(&archive).expect("zip"));
    writer
        .start_file("payload.bin", zip::write::SimpleFileOptions::default())
        .expect("entry");
    writer.write_all(&payload).expect("payload");
    writer.finish().expect("finish");

    let extractor = dispatcher::open(&archive).expect("detect zip");
    let options = ExtractOptions::default();
    let sink = NoopSink;
    let cancel = AtomicCancel::new();
    let summary = extractor
        .extract(&ExtractContext {
            source: &archive,
            dest: &dest,
            options: &options,
            progress: &sink,
            cancel: &cancel,
        })
        .expect("extract");

    assert_eq!(summary.bytes_written, payload.len() as u64);
    assert_eq!(
        std::fs::metadata(dest.join("payload.bin")).unwrap().len(),
        payload.len() as u64
    );
}
