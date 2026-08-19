//! macOS Finder 右键 Service（NSServices）provider。
//!
//! 与 Windows NSIS 右键菜单对齐：Finder 选中文件 → 右键 Services →
//! 「用 Extractr 解压到当前目录 / 解压到子目录 / 压缩」。
//!
//! 仅当 Extractr 运行时有效：app 启动在 setup 注册 provider，
//! app 退出后菜单项不可触发（本版接受，Quick Action .workflow 留后续）。
//!
//! Info.plist `NSServices` 数组声明 3 项，`NSMessage` = selector 全名，
//! 与下方 `#[unsafe(method(...))]` 字符串严格一致。

use std::sync::{Mutex, OnceLock};

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, NSObject};
use objc2::{define_class, msg_send, ClassType, MainThreadMarker};
use objc2_app_kit::{NSApplication, NSPasteboard};
use objc2_foundation::{NSArray, NSError, NSString, NSURL};
use tauri::{AppHandle, Emitter, Manager};

use crate::OpenAction;

/// provider 拿不到 AppHandle（selector 签名固定），setup 阶段写入，
/// selector 方法读取后 emit。setup 与 selector 均主线程，AppHandle Send+Clone。
static APP_HANDLE: Mutex<Option<AppHandle>> = Mutex::new(None);

/// provider 必须 Retained 保活：setServicesProvider 对 provider 是 weak 引用，
/// 这里持有强引用防止 drop 后菜单失效。
static PROVIDER: OnceLock<Retained<ServiceProvider>> = OnceLock::new();

define_class!(
    // SAFETY:
    // - NSObject 无 subclassing 要求。
    // - ServiceProvider 不实现 Drop。
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    struct ServiceProvider;

    impl ServiceProvider {
        // SAFETY: selector 签名 -(id)extractHere:(NSPasteboard*)pboard
        //                            userData:(NSString*)userData
        //                            error:(NSError**)err
        #[unsafe(method(extractHere:userData:error:))]
        fn extract_here(
            &self,
            pboard: &NSPasteboard,
            _user_data: &NSString,
            _error: *mut *mut NSError,
        ) -> *mut AnyObject {
            self.dispatch(pboard, ServiceKind::ExtractHere)
        }

        #[unsafe(method(extractToSubdir:userData:error:))]
        fn extract_to_subdir(
            &self,
            pboard: &NSPasteboard,
            _user_data: &NSString,
            _error: *mut *mut NSError,
        ) -> *mut AnyObject {
            self.dispatch(pboard, ServiceKind::ExtractToSubdir)
        }

        #[unsafe(method(compress:userData:error:))]
        fn compress(
            &self,
            pboard: &NSPasteboard,
            _user_data: &NSString,
            _error: *mut *mut NSError,
        ) -> *mut AnyObject {
            self.dispatch(pboard, ServiceKind::Compress)
        }
    }
);

#[derive(Clone, Copy)]
enum ServiceKind {
    ExtractHere,
    ExtractToSubdir,
    Compress,
}

impl ServiceProvider {
    /// 从 pasteboard 读 Finder 选中文件 URL → 构造 OpenAction →
    /// 写 PENDING_OPEN + emit "open-archive" 到 main 窗口。
    /// 静默失败（无文件/无 handle/无 path）时返回 nil，不设 error，
    /// 避免系统弹错；菜单项仍可用。
    fn dispatch(&self, pboard: &NSPasteboard, kind: ServiceKind) -> *mut AnyObject {
        let Some(path) = read_file_url(pboard) else {
            return std::ptr::null_mut();
        };
        let action = match kind {
            ServiceKind::ExtractHere => OpenAction::ExtractHere { path },
            ServiceKind::ExtractToSubdir => OpenAction::ExtractToSubdir { path },
            ServiceKind::Compress => OpenAction::Compress { path },
        };

        // 缓存 + emit。复用 lib.rs 的 PENDING_OPEN（前端 ready 后 pop）。
        if let Ok(mut slot) = crate::PENDING_OPEN.lock() {
            *slot = Some(action.clone());
        }
        if let Ok(guard) = APP_HANDLE.lock() {
            if let Some(handle) = guard.as_ref() {
                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.emit("open-archive", action);
                }
            }
        }
        std::ptr::null_mut()
    }

    /// alloc + init，按 hello_world_app.rs 模式。
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm);
        // SAFETY: NSObject init 签名正确。
        unsafe { msg_send![super(this), init] }
    }
}

/// 从 pasteboard 读首个 public.file-url NSURL，返回其本地 path。
fn read_file_url(pboard: &NSPasteboard) -> Option<String> {
    // readObjectsForClasses_options 接 &NSArray<AnyClass>。
    // from_slice 开的泛型 ObjectType 必须是 Message；AnyClass 实现了 Message。
    let classes = NSArray::<AnyClass>::from_slice(&[NSURL::class()]);
    // SAFETY: class_array 元素为 NSURL::class()，类型正确。
    let objs = unsafe { pboard.readObjectsForClasses_options(&classes, None) }?;
    let first = objs.firstObject()?;
    let url = first.downcast_ref::<NSURL>()?;
    let path = url.path()?;
    Some(path.to_string())
}

/// setup 阶段调用：缓存 AppHandle + 创建并保活 provider + 注册到 NSApp。
///
/// 必须在主线程调用（Tauri setup 即主线程）。
pub fn register_services_provider(handle: AppHandle) {
    // setup 在主线程；MainThreadMarker::new() 主线程返回 Some。
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };

    *APP_HANDLE.lock().unwrap() = Some(handle);

    let provider = ServiceProvider::new(mtm);
    // PROVIDER 持强引用保活；OnceLock::set 首次成功。
    let _ = PROVIDER.set(provider);

    let app = NSApplication::sharedApplication(mtm);
    // SAFETY: PROVIDER 已持有 provider，取 &* 传 setServicesProvider（weak）。
    if let Some(provider) = PROVIDER.get() {
        unsafe { app.setServicesProvider(Some(provider.as_ref())) };
    }
}
