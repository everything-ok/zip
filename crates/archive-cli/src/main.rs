//! Extractr 命令行接口。
//!
//! 子命令：extract / list / create / test / convert
//! 复用 archive-core 的解压/创建抽象。

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use archive_core::creators;
use archive_core::dispatcher;
use archive_core::progress::{AtomicCancel, NoopSink};
use archive_core::traits::{CreateContext, CreateOptions, CreateSource, ExtractContext};
use archive_core::{ExtractOptions, OverwritePolicy};

#[derive(Parser)]
#[command(name = "extractr-cli", version, about = "Extractr 命令行解压/压缩工具")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 解压归档
    Extract {
        /// 归档路径
        #[arg(short, long)]
        input: PathBuf,
        /// 输出目录
        #[arg(short, long)]
        output: PathBuf,
        /// 密码
        #[arg(short, long)]
        password: Option<String>,
        /// 覆盖策略：skip/overwrite/rename/error
        #[arg(short, long, default_value = "skip")]
        overwrite: String,
        /// 只解压这些条目（可多次）
        #[arg(long = "entry")]
        entries: Vec<String>,
    },
    /// 列出归档内容
    List {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        password: Option<String>,
    },
    /// 创建归档
    Create {
        /// 输出归档路径
        #[arg(short, long)]
        output: PathBuf,
        /// 源文件/目录（可多次）
        #[arg(short = 'F', long = "file")]
        sources: Vec<PathBuf>,
        /// 密码
        #[arg(short, long)]
        password: Option<String>,
        /// 压缩级别 0-9
        #[arg(short, long)]
        level: Option<i32>,
    },
    /// 测试归档完整性（CRC 校验）
    Test {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        password: Option<String>,
    },
    /// 格式转换
    Convert {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long)]
        password: Option<String>,
        #[arg(long = "dest-password")]
        dest_password: Option<String>,
        #[arg(short, long)]
        level: Option<i32>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Extract {
            input,
            output,
            password,
            overwrite,
            entries,
        } => {
            let extractor = dispatcher::open(&input)?;
            let opts = ExtractOptions {
                password,
                overwrite: parse_overwrite(&overwrite),
                ..Default::default()
            };
            let cancel = AtomicCancel::new();
            let ctx = ExtractContext {
                source: &input,
                dest: &output,
                options: &opts,
                progress: &NoopSink,
                cancel: &cancel,
            };
            let summary = if entries.is_empty() {
                extractor.extract(&ctx)?
            } else {
                extractor.extract_entries(&ctx, &entries)?
            };
            println!(
                "已解压 {} 项，{} 字节",
                summary.entries_extracted, summary.bytes_written
            );
            Ok(())
        }
        Commands::List { input, password } => {
            let extractor = dispatcher::open(&input)?;
            let entries = extractor.list(&input, password.as_deref())?;
            println!("{:<8} {:<10} 路径", "类型", "大小");
            for e in entries {
                let kind = if e.is_dir { "目录" } else { "文件" };
                println!("{:<8} {:<10} {}", kind, e.size, e.path);
            }
            Ok(())
        }
        Commands::Create {
            output,
            sources,
            password,
            level,
        } => {
            let create_sources: Vec<CreateSource> = sources
                .iter()
                .map(|p| {
                    let name = p
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("file")
                        .to_string();
                    CreateSource {
                        fs_path: p.clone(),
                        archive_path: name,
                    }
                })
                .collect();
            let creator = creators::creator_for_path(&output)?;
            let opts = CreateOptions { password, level };
            let cancel = AtomicCancel::new();
            let ctx = CreateContext {
                dest: &output,
                sources: &create_sources,
                options: &opts,
                progress: &NoopSink,
                cancel: &cancel,
            };
            let summary = creator.create(&ctx)?;
            println!(
                "已创建 {} 项，{} 字节",
                summary.entries_extracted, summary.bytes_written
            );
            Ok(())
        }
        Commands::Test { input, password } => {
            let extractor = dispatcher::open(&input)?;
            let opts = ExtractOptions {
                password,
                ..Default::default()
            };
            let cancel = AtomicCancel::new();
            let ctx = ExtractContext {
                source: &input,
                dest: std::path::Path::new(""),
                options: &opts,
                progress: &NoopSink,
                cancel: &cancel,
            };
            extractor.test(&ctx)?;
            println!("归档完整性校验通过");
            Ok(())
        }
        Commands::Convert {
            input,
            output,
            password,
            dest_password,
            level,
        } => {
            let tmp = tempfile_dir()?;
            let extractor = dispatcher::open(&input)?;
            let src_opts = ExtractOptions {
                password,
                overwrite: OverwritePolicy::Overwrite,
                ..Default::default()
            };
            let cancel = AtomicCancel::new();
            extractor.extract(&ExtractContext {
                source: &input,
                dest: &tmp,
                options: &src_opts,
                progress: &NoopSink,
                cancel: &cancel,
            })?;
            let mut create_sources = Vec::new();
            collect(&tmp, "", &mut create_sources)?;
            let creator = creators::creator_for_path(&output)?;
            let opts = CreateOptions {
                password: dest_password,
                level,
            };
            creator.create(&CreateContext {
                dest: &output,
                sources: &create_sources,
                options: &opts,
                progress: &NoopSink,
                cancel: &cancel,
            })?;
            let _ = std::fs::remove_dir_all(&tmp);
            println!("转换完成");
            Ok(())
        }
    }
}

fn parse_overwrite(s: &str) -> OverwritePolicy {
    match s {
        "overwrite" => OverwritePolicy::Overwrite,
        "rename" => OverwritePolicy::Rename,
        "error" => OverwritePolicy::Error,
        _ => OverwritePolicy::Skip,
    }
}

fn tempfile_dir() -> Result<PathBuf> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("extractr-cli-{}-{}", std::process::id(), seq));
    std::fs::create_dir_all(&p)?;
    Ok(p)
}

fn collect(
    base: &std::path::Path,
    prefix: &str,
    out: &mut Vec<CreateSource>,
) -> Result<()> {
    for entry in std::fs::read_dir(base)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let archive_path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        if entry.file_type()?.is_dir() {
            collect(&path, &archive_path, out)?;
        } else {
            out.push(CreateSource {
                fs_path: path,
                archive_path,
            });
        }
    }
    Ok(())
}
