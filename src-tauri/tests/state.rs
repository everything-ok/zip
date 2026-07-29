//! src-tauri 命令层单元测试。

#![cfg(test)]

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use extractr_lib::state::{AppState, RegisterError};

#[tokio::test]
async fn register_duplicate_task_rejected() {
    let state = AppState::new();
    let token = Arc::new(AtomicBool::new(false));
    let first = state.register_task("id-1", token.clone()).await;
    assert!(first.is_ok());
    let second = state.register_task("id-1", token.clone()).await;
    assert!(matches!(second, Err(RegisterError::Duplicate(_))));
}

#[tokio::test]
async fn drop_task_only_removes_matching() {
    let state = AppState::new();
    let t1 = Arc::new(AtomicBool::new(false));
    let t2 = Arc::new(AtomicBool::new(false));
    state.register_task("id-a", t1.clone()).await.unwrap();
    state.register_task("id-a-replaced", t2.clone()).await.unwrap();
    // 用 t1 调 drop_task 对 "id-a-replaced"：指针不等，不应删除。
    state.drop_task("id-a-replaced", &t1).await;
    // 再注册应仍被占（id-a-replaced 还在）。
    let dup = state.register_task("id-a-replaced", t1.clone()).await;
    assert!(matches!(dup, Err(RegisterError::Duplicate(_))));
}

#[tokio::test]
async fn concurrent_limit_enforced() {
    let state = AppState::new();
    for i in 0..extractr_lib::state::MAX_CONCURRENT_TASKS {
        let tok = Arc::new(AtomicBool::new(false));
        let id = format!("job-{i}");
        assert!(state.register_task(&id, tok).await.is_ok());
    }
    // 超限应拒绝。
    let over = state
        .register_task("overflow", Arc::new(AtomicBool::new(false)))
        .await;
    assert!(matches!(over, Err(RegisterError::TooMany(_))));
}
