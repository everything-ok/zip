import UIAbility from '@ohos.app.ability.UIAbility';
import window from '@ohos.window';
import { CoreBridge } from '../bridge/CoreBridge';

export default class EntryAbility extends UIAbility {
  onCreate(want, launchParam) {
    console.info('[Extractr] EntryAbility onCreate');

    // 处理文件关联启动
    if (want.parameters && want.parameters['ohos.extra.param.key.contentUri']) {
      const uri = want.parameters['ohos.extra.param.key.contentUri'];
      AppStorage.setOrCreate('pendingOpenUri', uri);
    }
  }

  onWindowStageCreate(windowStage: window.WindowStage) {
    console.info('[Extractr] onWindowStageCreate');

    windowStage.loadContent('pages/Index', (err, data) => {
      if (err.code) {
        console.error('[Extractr] Failed to load content: ' + JSON.stringify(err));
        return;
      }
      console.info('[Extractr] Content loaded successfully');
    });
  }

  onNewWant(want, launchParam) {
    // 处理从文件管理器再次打开
    if (want.parameters && want.parameters['ohos.extra.param.key.contentUri']) {
      const uri = want.parameters['ohos.extra.param.key.contentUri'];
      AppStorage.setOrCreate('pendingOpenUri', uri);
    }
  }
}
