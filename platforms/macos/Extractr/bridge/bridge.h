// archive-core C ABI 导出头文件
// 由 cbindgen 从 Rust crate archive-core 自动生成
// 此文件为占位，实际构建时由 build.rs 调用 cbindgen 覆盖

#ifndef ARCHIVE_CORE_BRIDGE_H
#define ARCHIVE_CORE_BRIDGE_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/// 探测归档格式，返回格式字符串（需调用方 free）
/// 返回值: "zip" | "7z" | "rar" | "tar" | "gzip" | "bzip2" | "xz" | "zstd" | "tar.gz" | ...
/// 失败返回 NULL
char* archive_detect_format(const char* path);

/// 列出归档条目
/// 返回 0 成功，非 0 失败
/// out_entries 需调用 archive_free_entry_list 释放
int32_t archive_list(
    const char* path,
    const char* password,
    ArchiveEntry** out_entries,
    int32_t* out_count
);

/// 解压归档
/// progress_cb: void callback(int32_t kind, int32_t current, uint64_t processed, uint64_t total)
/// 返回 0 成功，非 0 失败
int32_t archive_extract(
    const char* source,
    const char* dest,
    const char* password,
    int32_t overwrite_policy,
    void (*progress_cb)(int32_t, int32_t, uint64_t, uint64_t),
    void* progress_user_data
);

/// 创建归档
int32_t archive_create(
    const char* dest,
    const CreateSourceC* sources,
    int32_t source_count,
    const char* password,
    int32_t level,
    void (*progress_cb)(int32_t, int32_t, uint64_t, uint64_t),
    void* progress_user_data
);

/// 释放 archive_list 返回的条目列表
void archive_free_entry_list(ArchiveEntry* entries, int32_t count);

/// 释放 archive_detect_format 返回的字符串
void archive_free_string(char* s);

// 数据结构

typedef struct {
    char* path;
    uint64_t size;
    uint64_t compressed_size;
    bool is_dir;
    bool is_encrypted;
    int64_t modified;  // unix seconds, -1 = unknown
} ArchiveEntry;

typedef struct {
    const char* fs_path;
    const char* archive_path;
} CreateSourceC;

#ifdef __cplusplus
}
#endif

#endif // ARCHIVE_CORE_BRIDGE_H
