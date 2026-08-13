// archive-core NAPI 桥接层
// 封装 Rust cdylib 导出函数为 ArkTS 可调用的异步 API

import { NAPI } from './napi_loader';

export interface ArchiveEntry {
  path: string;
  size: number;
  compressedSize: number;
  isDir: boolean;
  isEncrypted: boolean;
  modified: number | null;
}

export interface ExtractSummary {
  entriesExtracted: number;
  entriesSkipped: number;
  bytesWritten: number;
  cancelled: boolean;
}

export type OverwritePolicy = 'skip' | 'overwrite' | 'rename' | 'error';

export class CoreBridge {
  private static instance: CoreBridge | null = null;
  private nativeLib: object | null = null;

  private constructor() {
    this.nativeLib = NAPI.load('archive_core_napi');
  }

  static getInstance(): CoreBridge {
    if (!CoreBridge.instance) {
      CoreBridge.instance = new CoreBridge();
    }
    return CoreBridge.instance;
  }

  /// 探测归档格式
  async detectFormat(path: string): Promise<string> {
    // TODO: 调用 NAPI archive_detect_format
    return 'unknown';
  }

  /// 列出归档条目
  async listArchive(path: string, password?: string): Promise<ArchiveEntry[]> {
    // TODO: 调用 NAPI archive_list
    return [];
  }

  /// 解压归档
  async extract(
    source: string,
    dest: string,
    password?: string,
    overwrite: OverwritePolicy = 'skip',
    onProgress?: (progress: number) => void
  ): Promise<ExtractSummary> {
    // TODO: 调用 NAPI archive_extract
    return {
      entriesExtracted: 0,
      entriesSkipped: 0,
      bytesWritten: 0,
      cancelled: false,
    };
  }

  /// 压缩创建归档
  async create(
    dest: string,
    sources: Array<{ fsPath: string; archivePath: string }>,
    password?: string,
    level?: number
  ): Promise<ExtractSummary> {
    // TODO: 调用 NAPI archive_create
    return {
      entriesExtracted: 0,
      entriesSkipped: 0,
      bytesWritten: 0,
      cancelled: false,
    };
  }
}
