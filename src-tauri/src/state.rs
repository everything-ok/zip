//! 全局应用状态：进行中的任务表与取消令牌。

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tokio::sync::Mutex;

/// 并发解压任务上限。超过则拒绝新任务，防资源耗尽与磁盘抖动。
pub const MAX_CONCURRENT_TASKS: usize = 3;

pub struct AppState {
    /// task_id -> 取消标志。前端调 `cancel_extraction` 置位。
    pub tasks: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(HashMap::new()),
        }
    }

    /// 注册一个任务令牌。
    ///
    /// 返回：
    /// - `Ok(token)`：注册成功；
    /// - `Err(RegisterError::Duplicate(existing))`：task_id 已存在；
    /// - `Err(RegisterError::TooMany(current))`：并发任务已达上限。
    pub async fn register_task(
        &self,
        task_id: &str,
        token: Arc<AtomicBool>,
    ) -> Result<Arc<AtomicBool>, RegisterError> {
        let mut tasks = self.tasks.lock().await;
        if let Some(existing) = tasks.get(task_id) {
            return Err(RegisterError::Duplicate(existing.clone()));
        }
        if tasks.len() >= MAX_CONCURRENT_TASKS {
            return Err(RegisterError::TooMany(tasks.len()));
        }
        tasks.insert(task_id.to_string(), token.clone());
        Ok(token)
    }

    /// 清理任务令牌，仅当 map 中仍是同一个 `Arc`（指针相等）时才删除，
    /// 避免先结束的旧任务误删后注册的新任务令牌。
    pub async fn drop_task(&self, task_id: &str, expected: &Arc<AtomicBool>) {
        let mut tasks = self.tasks.lock().await;
        if let Some(current) = tasks.get(task_id) {
            if Arc::ptr_eq(current, expected) {
                tasks.remove(task_id);
            }
        }
    }

    /// 置位指定任务的取消标志。
    pub async fn cancel_task(&self, task_id: &str) {
        if let Some(token) = self.tasks.lock().await.get(task_id) {
            token.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }
}

/// 注册任务失败原因。
pub enum RegisterError {
    /// task_id 已存在（旧任务令牌）。
    Duplicate(Arc<AtomicBool>),
    /// 并发任务已达上限。
    TooMany(usize),
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
