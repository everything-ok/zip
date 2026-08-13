import Foundation

// MARK: - FFI 桥接层
// 调用 archive-core cdylib 导出的 C 函数。
// 头文件由 cbindgen 从 Rust 生成（bridge.h）。

/// archive-core C API 桥接
class CoreBridge {
    // MARK: 单例
    static let shared = CoreBridge()

    private init() {
        // dylib 在 app bundle 的 Frameworks 目录，dlopen 加载
        let bundlePath = Bundle.main.bundleURL
            .appendingPathComponent("Contents/Frameworks/libarchive_core.dylib")
            .path
        if let path = bundlePath {
            _ = dlopen(path, RTLD_NOW)
        }
    }

    // MARK: C 函数指针类型定义
    // 这些函数签名需与 Rust #[no_mangle] export 对齐

    typedefalias func DetectFormatFunc = @convention(c) (UnsafePointer<CChar>) -> UnsafePointer<CChar>?
    typedefalias func ListArchiveFunc = @convention(c) (
        UnsafePointer<CChar>,  // path
        UnsafePointer<CChar>?, // password
        UnsafeMutablePointer<EntryList>?
    ) -> Int32  // 0 = success

    typedefalias func ExtractArchiveFunc = @convention(c) (
        UnsafePointer<CChar>,  // source
        UnsafePointer<CChar>,  // dest
        UnsafePointer<CChar>?, // password
        Int32,                 // overwrite policy
        @convention(c) (Int32, Int32, UInt64, UInt64) -> Void  // progress callback
    ) -> Int32

    // TODO: 加载函数指针并封装为 Swift async API

    /// 探测归档格式
    func detectFormat(path: String) async throws -> String {
        // TODO: 实现
        fatalError("未实现")
    }

    /// 列出归档条目
    func listArchive(path: String, password: String?) async throws -> [ArchiveEntry] {
        // TODO: 实现
        fatalError("未实现")
    }

    /// 解压归档
    func extract(
        source: String,
        dest: String,
        password: String?,
        overwrite: OverwritePolicy,
        onProgress: ((Double) -> Void)?
    ) async throws -> ExtractSummary {
        // TODO: 实现
        fatalError("未实现")
    }

    /// 压缩创建归档
    func create(
        dest: String,
        sources: [CreateSource],
        password: String?,
        level: Int32
    ) async throws -> ExtractSummary {
        // TODO: 实现
        fatalError("未实现")
    }
}

// MARK: - FFI 数据结构

struct EntryList {
    var entries: UnsafePointer<FFIEntry>?
    var count: Int32
}

struct FFIEntry {
    var path: UnsafePointer<CChar>?
    var size: UInt64
    var compressed_size: UInt64
    var is_dir: Bool
    var is_encrypted: Bool
    var modified: Int64  // unix seconds, -1 = unknown
}

struct CreateSource {
    var fsPath: String
    var archivePath: String
}

enum OverwritePolicy: Int32 {
    case skip = 0
    case overwrite = 1
    case rename = 2
    case error = 3
}

struct ExtractSummary {
    var entriesExtracted: Int
    var entriesSkipped: Int
    var bytesWritten: UInt64
    var cancelled: Bool
}
