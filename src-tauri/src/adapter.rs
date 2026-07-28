//! 把 Tauri `Channel<ProgressEvent>` 适配成 archive-core 的 `ProgressSink`。
//! `on_progress` 节流（每 40ms 最多一次）防通道过载；并在此层计算速度与 ETA。

use std::sync::Mutex as StdMutex;
use std::time::{Duration, Instant};

use tauri::ipc::Channel;

use archive_core::traits::ProgressSink;
use archive_core::types::ArchiveEntry;

use crate::events::ProgressEvent;

pub struct ChannelSink {
    ch: Channel<ProgressEvent>,
    last_bytes_emit: StdMutex<Option<Instant>>,
    // 速度计算状态：上次进度的时间与已处理字节。
    speed_state: StdMutex<Option<(Instant, u64)>>,
}

impl ChannelSink {
    pub fn new(ch: Channel<ProgressEvent>) -> Self {
        Self {
            ch,
            last_bytes_emit: StdMutex::new(None),
            speed_state: StdMutex::new(None),
        }
    }

    fn emit(&self, ev: ProgressEvent) {
        let _ = self.ch.send(ev);
    }
}

impl ProgressSink for ChannelSink {
    fn on_start(&self, total_entries: usize, total_bytes: u64) {
        // 任务开始时重置速度基线。
        *self.speed_state.lock().unwrap() = Some((Instant::now(), 0));
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

            // 计算速度与 ETA。
            let (speed, eta) = self.compute_speed(processed, total, now);
            if let Some(bps) = speed {
                self.emit(ProgressEvent::Bytes {
                    processed,
                    total,
                    indeterminate: total == 0,
                    speed: bps,
                    eta_secs: eta,
                });
            } else {
                self.emit(ProgressEvent::Bytes {
                    processed,
                    total,
                    indeterminate: total == 0,
                    speed: 0,
                    eta_secs: None,
                });
            }
        }
    }
}

impl ChannelSink {
    /// 基于累计字节的瞬时速度（字节/秒）与 ETA。
    fn compute_speed(
        &self,
        processed: u64,
        total: u64,
        now: Instant,
    ) -> (Option<u64>, Option<u64>) {
        let mut state = self.speed_state.lock().unwrap();
        let prev = state.unwrap_or((now, 0));
        let elapsed = now.duration_since(prev.0).as_secs_f64();
        if elapsed < 0.2 {
            // 间隔过短不更新基线，返回上次速度（None 表示暂不报速度）。
            *state = Some(prev);
            return (None, None);
        }
        let delta_bytes = processed.saturating_sub(prev.1);
        let bps = if elapsed > 0.0 {
            (delta_bytes as f64 / elapsed) as u64
        } else {
            0
        };
        let eta = if total > 0 && bps > 0 {
            total.saturating_sub(processed).checked_div(bps)
        } else {
            None
        };
        *state = Some((now, processed));
        (Some(bps), eta)
    }
}
