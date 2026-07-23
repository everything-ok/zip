//! 把 Tauri `Channel<ProgressEvent>` 适配成 archive-core 的 `ProgressSink`。
//! `on_bytes` 节流（每 40ms 最多一次）防通道过载。

use std::sync::Mutex as StdMutex;
use std::time::{Duration, Instant};

use tauri::ipc::Channel;

use archive_core::traits::ProgressSink;
use archive_core::types::ArchiveEntry;

use crate::events::ProgressEvent;

pub struct ChannelSink {
    ch: Channel<ProgressEvent>,
    last_bytes_emit: StdMutex<Option<Instant>>,
}

impl ChannelSink {
    pub fn new(ch: Channel<ProgressEvent>) -> Self {
        Self {
            ch,
            last_bytes_emit: StdMutex::new(None),
        }
    }

    fn emit(&self, ev: ProgressEvent) {
        let _ = self.ch.send(ev);
    }
}

impl ProgressSink for ChannelSink {
    fn on_start(&self, total_entries: usize, total_bytes: u64) {
        self.emit(ProgressEvent::Started {
            total_entries,
            total_bytes,
        });
    }

    fn on_entry_start(&self, idx: usize, total: usize, e: &ArchiveEntry) {
        self.emit(ProgressEvent::EntryStart {
            index: idx,
            total,
            path: e.path.clone(),
            size: e.size,
        });
    }

    fn on_entry_done(&self, idx: usize, _bytes_written: u64) {
        self.emit(ProgressEvent::EntryDone { index: idx });
    }

    fn on_progress(&self, processed: u64, total: u64) {
        let mut last = self.last_bytes_emit.lock().unwrap();
        let now = Instant::now();
        let should = last
            .map(|t| now.duration_since(t) >= Duration::from_millis(40))
            .unwrap_or(true);
        if should {
            *last = Some(now);
            drop(last);
            self.emit(ProgressEvent::Bytes {
                processed,
                total,
                indeterminate: total == 0,
            });
        }
    }
}
