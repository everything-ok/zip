//! src-tauri 命令层与错误映射单元测试。
//!
//! `parse_open_action` 与 `ArchiveErrorDto::from_anyhow` 是纯逻辑，不依赖 Tauri 运行时，
//! 提取为独立可测函数。这里通过 `extractr_lib` crate 内联测试覆盖。

use extractr_lib::events::ArchiveErrorDto;

#[test]
fn from_anyhow_password_required() {
    let err = anyhow::anyhow!("归档已加密，需要密码");
    let dto = ArchiveErrorDto::from_anyhow(&err);
    // 字符串兜底命中 "密码" 关键词。
    assert_eq!(dto.code, "password_required");
}

#[test]
fn from_anyhow_concrete_archive_error() {
    use archive_core::error::ArchiveError;
    let err: anyhow::Error = ArchiveError::WrongPassword.into();
    let dto = ArchiveErrorDto::from_anyhow(&err);
    assert_eq!(dto.code, "wrong_password");
}

#[test]
fn from_anyhow_bomb_detected() {
    use archive_core::error::ArchiveError;
    let err: anyhow::Error = ArchiveError::BombDetected {
        current: 9999,
        max: 1000,
    }
    .into();
    let dto = ArchiveErrorDto::from_anyhow(&err);
    assert_eq!(dto.code, "bomb_detected");
}

#[test]
fn from_anyhow_cancelled() {
    use archive_core::error::ArchiveError;
    let err: anyhow::Error = ArchiveError::Cancelled.into();
    let dto = ArchiveErrorDto::from_anyhow(&err);
    assert_eq!(dto.code, "cancelled");
}
