// NAPI 原生库加载器
// 鸿蒙通过 @ohos.napi 加载 .so 原生库

export class NAPI {
  static load(libName: string): object | null {
    try {
      // @ts-ignore — 鸿蒙 NAPI 加载 API
      const lib = globalThis.requireNapi(libName);
      return lib;
    } catch (e) {
      console.error(`[NAPI] Failed to load ${libName}: ${e}`);
      return null;
    }
  }
}
