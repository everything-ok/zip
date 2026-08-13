import SwiftUI

@main
struct ExtractrApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
        .commands {
            // 注册文件打开命令
            CommandGroup(after: .newItem) {
                Button("Open Archive...") {
                    openFilePicker()
                }
                .keyboardShortcut("o", modifiers: .command)
            }
        }
    }

    private func openFilePicker() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.zip, .gzip, .tar]
        panel.allowsMultipleSelection = false
        panel.begin { response in
            guard response == .OK, let url = panel.urls.first else { return }
            // TODO: 传递文件路径到 ContentView
        }
    }
}
