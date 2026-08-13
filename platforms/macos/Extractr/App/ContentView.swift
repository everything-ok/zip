import SwiftUI

struct ContentView: View {
    @State private var filePath: String?
    @State private var entries: [ArchiveEntry] = []
    @State private var destPath: String = ""
    @State private var isExtracting: Bool = false
    @State private var progress: Double = 0

    var body: some View {
        VStack(spacing: 16) {
            if let path = filePath {
                // 归档预览
                ArchivePreviewView(
                    path: path,
                    entries: entries,
                    onExtract: { extractArchive() }
                )
            } else {
                // 拖拽区域
                DropZoneView(onFile: { path in
                    loadArchive(path)
                })
            }
        }
        .frame(minWidth: 720, minHeight: 520)
        .padding()
    }

    private func loadArchive(_ path: String) {
        filePath = path
        // TODO: 调用 CoreBridge.list()
    }

    private func extractArchive() {
        guard let source = filePath, !destPath.isEmpty else { return }
        isExtracting = true
        // TODO: 调用 CoreBridge.extract()
    }
}

// MARK: - 数据模型

struct ArchiveEntry: Identifiable {
    let id = UUID()
    let path: String
    let size: UInt64
    let compressedSize: UInt64
    let isDir: Bool
    let isEncrypted: Bool
    let modified: Date?
}

// MARK: - 子视图（占位）

struct DropZoneView: View {
    let onFile: (String) -> Void

    var body: some View {
        Text("拖拽压缩文件到此处")
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Color(nsColor: .controlBackgroundColor))
            .cornerRadius(12)
            .onDrop(of: [.fileURL], isTargeted: nil) { providers in
                // TODO: 处理拖拽
                false
            }
    }
}

struct ArchivePreviewView: View {
    let path: String
    let entries: [ArchiveEntry]
    let onExtract: () -> Void

    var body: some View {
        Text("预览: \(path)")
        // TODO: 完整预览界面
    }
}
