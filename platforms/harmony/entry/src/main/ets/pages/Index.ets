@Entry
@Component
struct Index {
  @State filePath: string = '';
  @State entries: ArchiveEntry[] = [];
  @State destPath: string = '';
  @State isExtracting: boolean = false;
  @State progress: number = 0;
  @State hasFile: boolean = false;

  build() {
    Column() {
      // 标题栏
      Row() {
        Image($r('app.media.app_icon'))
          .width(32)
          .height(32)
          .borderRadius(8)
        Text('Extractr')
          .fontSize(16)
          .fontWeight(FontWeight.Bold)
          .margin({ left: 8 })
        Text('轻量解压工具')
          .fontSize(12)
          .fontColor('#999999')
          .margin({ left: 4 })
        Blank()
        // 操作按钮
        Button($r('app.string.compress'))
          .fontSize(12)
          .height(32)
          .onClick(() => {
            // TODO: 打开压缩面板
          })
      }
      .width('100%')
      .height(56)
      .padding({ left: 16, right: 16 })
      .alignItems(VerticalAlign.Center)

      Divider()

      if (this.hasFile) {
        // 归档预览
        this.ArchivePreview()
      } else {
        // 拖拽区域
        this.DropZone()
      }
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#F5F5F5')
  }

  @Builder
  DropZone() {
    Column() {
      Text('拖拽压缩文件到此处')
        .fontSize(16)
        .fontColor('#666666')
      Text('或点击选择文件')
        .fontSize(12)
        .fontColor('#999999')
        .margin({ top: 8 })
    }
    .width('100%')
    .layoutWeight(1)
    .justifyContent(FlexAlign.Center)
    .alignItems(HorizontalAlign.Center)
    .borderWidth(2)
    .borderStyle(BorderStyle.Dashed)
    .borderColor('#C7D2FE')
    .borderRadius(12)
    .margin(16)
    .onClick(() => {
      // TODO: 文件选择器
    })
  }

  @Builder
  ArchivePreview() {
    Column() {
      Text(this.filePath)
        .fontSize(14)
        .maxLines(1)
        .textOverflow({ overflow: TextOverflow.Ellipsis })
        .width('100%')
        .padding(16)

      // 文件列表
      List() {
        ForEach(this.entries, (entry: ArchiveEntry) => {
          ListItem() {
            Row() {
              Text(entry.path)
                .fontSize(12)
                .layoutWeight(1)
              Text(entry.isDir ? '-' : formatBytes(entry.size))
                .fontSize(10)
                .fontColor('#999999')
            }
            .width('100%')
            .padding({ left: 16, right: 16, top: 8, bottom: 8 })
          }
        })
      }
      .layoutWeight(1)
      .width('100%')

      // 解压按钮
      Row() {
        Button('解压')
          .width(120)
          .height(40)
          .enabled(!this.isExtracting && this.destPath.length > 0)
          .onClick(() => {
            this.doExtract();
          })
      }
      .width('100%')
      .justifyContent(FlexAlign.End)
      .padding(16)
    }
    .width('100%')
    .layoutWeight(1)
  }

  private async doExtract() {
    this.isExtracting = true;
    try {
      const bridge = CoreBridge.getInstance();
      const summary = await bridge.extract(this.filePath, this.destPath);
      // TODO: 处理结果
    } catch (e) {
      console.error('[Extractr] Extract failed: ' + e);
    } finally {
      this.isExtracting = false;
    }
  }
}

// 工具函数
function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return (bytes / Math.pow(1024, i)).toFixed(1) + ' ' + units[i];
}

// 数据模型
interface ArchiveEntry {
  path: string;
  size: number;
  compressedSize: number;
  isDir: boolean;
  isEncrypted: boolean;
  modified: number | null;
}

import { CoreBridge } from '../bridge/CoreBridge';
