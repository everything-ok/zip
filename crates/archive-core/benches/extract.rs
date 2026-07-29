//! 解压吞吐量基准：ZIP / 7z / tar.gz。
//! `cargo bench -p archive-core` 运行。

use std::io::Write;

use archive_core::dispatcher;
use archive_core::progress::{AtomicCancel, NoopSink};
use archive_core::traits::ExtractContext;
use archive_core::ExtractOptions;
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_zip(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let arch = tmp.path().join("bench.zip");
    let payload = vec![0x5Au8; 4 * 1024 * 1024]; // 4 MiB
    let mut w = zip::ZipWriter::new(std::fs::File::create(&arch).unwrap());
    w.start_file("p.bin", zip::write::SimpleFileOptions::default()).unwrap();
    w.write_all(&payload).unwrap();
    w.finish().unwrap();
    let dest = tmp.path().join("out");

    c.bench_function("extract/zip_4mib", |b| {
        b.iter(|| {
            let d = dest.join("run");
            let extractor = dispatcher::open(&arch).unwrap();
            extractor
                .extract(&ExtractContext {
                    source: &arch,
                    dest: &d,
                    options: &ExtractOptions::default(),
                    progress: &NoopSink,
                    cancel: &AtomicCancel::new(),
                })
                .unwrap();
            let _ = std::fs::remove_dir_all(&d);
        });
    });
}

fn bench_tar_gz(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let payload = vec![0x5Au8; 2 * 1024 * 1024];
    std::fs::write(src.join("big.bin"), &payload).unwrap();
    let arch = tmp.path().join("bench.tar.gz");
    let gz = flate2::write::GzEncoder::new(std::fs::File::create(&arch).unwrap(), flate2::Compression::default());
    let mut builder = tar::Builder::new(gz);
    builder.append_path_with_name(src.join("big.bin"), "big.bin").unwrap();
    builder.into_inner().unwrap().finish().unwrap();
    let dest = tmp.path().join("out");

    c.bench_function("extract/tar_gz_2mib", |b| {
        b.iter(|| {
            let d = dest.join("run");
            let extractor = dispatcher::open(&arch).unwrap();
            extractor
                .extract(&ExtractContext {
                    source: &arch,
                    dest: &d,
                    options: &ExtractOptions::default(),
                    progress: &NoopSink,
                    cancel: &AtomicCancel::new(),
                })
                .unwrap();
            let _ = std::fs::remove_dir_all(&d);
        });
    });
}

criterion_group!(benches, bench_zip, bench_tar_gz);
criterion_main!(benches);
