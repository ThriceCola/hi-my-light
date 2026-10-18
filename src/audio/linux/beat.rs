//! 在抓音频的线程上跑 BeatNet。模型只在打开节拍开关后加载。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use beatnet_rs::{BeatNet, BeatNetConfig};

use super::{CHANNELS, RATE};

pub struct BeatCtl {
    pub on: AtomicBool,
    pub ready: AtomicBool,
    reset: AtomicBool,
    error: Mutex<Option<String>>,
}

impl BeatCtl {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            on: AtomicBool::new(false),
            ready: AtomicBool::new(false),
            reset: AtomicBool::new(false),
            error: Mutex::new(None),
        })
    }

    pub fn request_reset(&self) {
        self.reset.store(true, Ordering::Relaxed);
    }

    fn take_reset(&self) -> bool {
        self.reset.swap(false, Ordering::Relaxed)
    }

    pub fn set_on(&self, on: bool) {
        self.on.store(on, Ordering::Relaxed);
    }

    pub fn ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    pub fn error(&self) -> Option<String> {
        self.error.lock().expect("beat error").clone()
    }

    fn fail(&self, message: String) {
        self.ready.store(false, Ordering::Relaxed);
        *self.error.lock().expect("beat error") = Some(message);
    }

    fn loaded(&self) {
        self.ready.store(true, Ordering::Relaxed);
        *self.error.lock().expect("beat error") = None;
    }
}

pub struct BeatFollow {
    ctl: Arc<BeatCtl>,
    net: Option<BeatNet>,
    retry_at: Option<Instant>,
    cursor: u64,
    gap: bool,
    pcm: Vec<f32>,
}

impl BeatFollow {
    pub fn new(ctl: Arc<BeatCtl>) -> Self {
        Self {
            ctl,
            net: None,
            retry_at: None,
            cursor: 0,
            gap: false,
            pcm: Vec::new(),
        }
    }

    pub fn stall(&mut self) {
        self.gap = true;
    }

    /// 返回 (约 30ms 尾部 RMS, 重拍, 弱拍, 这一拍相对音频末尾的延迟毫秒)。
    pub fn feed(&mut self, bytes: &[u8]) -> (f32, bool, bool, Option<u32>) {
        decode(bytes, &mut self.pcm);
        let rms_fast = tail_rms(&self.pcm);
        if !self.ctl.on.load(Ordering::Relaxed) {
            self.gap = true;
            self.cursor = self.cursor.saturating_add(frames(&self.pcm));
            return (rms_fast, false, false, None);
        }
        if self.net.is_none() {
            self.ensure_model();
        }
        let Some(net) = self.net.as_mut() else {
            self.cursor = self.cursor.saturating_add(frames(&self.pcm));
            self.gap = true;
            return (rms_fast, false, false, None);
        };
        if self.ctl.take_reset() {
            self.gap = true;
        }
        if self.gap {
            net.discontinuity(self.cursor);
            self.gap = false;
        }
        if self.pcm.iter().any(|sample| !sample.is_finite()) {
            self.gap = true;
            self.cursor = self.cursor.saturating_add(frames(&self.pcm));
            return (rms_fast, false, false, None);
        }
        let mut downbeat = false;
        let mut beat = false;
        let mut event_at: Option<f64> = None;
        let mut lag = None;
        match net.process(&self.pcm, |frame| {
            if let Some(event) = frame.event {
                event_at = Some(event.time);
                if event.downbeat {
                    downbeat = true;
                } else {
                    beat = true;
                }
            }
        }) {
            Ok(()) => {
                self.cursor = self.cursor.saturating_add(frames(&self.pcm));
                lag = event_at.map(|time| {
                    let end = self.cursor as f64 / f64::from(RATE);
                    ((end - time) * 1000.0).max(0.0).round() as u32
                });
            }
            Err(_) => {
                net.reset();
                self.cursor = 0;
                self.gap = false;
            }
        }
        (rms_fast, downbeat, beat, lag)
    }

    fn ensure_model(&mut self) {
        if self
            .retry_at
            .is_some_and(|at| at.elapsed() < Duration::from_secs(3))
        {
            return;
        }
        match BeatNet::new(BeatNetConfig {
            sample_rate: RATE,
            channels: u16::from(CHANNELS),
            ..BeatNetConfig::default()
        }) {
            Ok(net) => {
                self.net = Some(net);
                self.gap = true;
                self.ctl.loaded();
            }
            Err(err) => {
                self.retry_at = Some(Instant::now());
                self.gap = true;
                self.ctl.fail(err.to_string());
            }
        }
    }
}

fn frames(pcm: &[f32]) -> u64 {
    (pcm.len() / usize::from(CHANNELS)) as u64
}

fn decode(bytes: &[u8], out: &mut Vec<f32>) {
    out.clear();
    out.reserve(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        out.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
}

fn tail_rms(pcm: &[f32]) -> f32 {
    let window = (RATE as usize) * 30 / 1000 * usize::from(CHANNELS);
    let start = pcm.len().saturating_sub(window);
    super::super::rms_f32(&pcm[start..])
}
