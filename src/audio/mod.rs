//! 电脑正在播放的声音（不是麦克风）。
//!
//! 各系统抓法差很多，实现按操作系统分目录、用 `cfg` 选进去。
//! 调用方只看有没有声、有多响。

use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod stub;

#[cfg(target_os = "linux")]
use linux as imp;
#[cfg(target_os = "windows")]
use windows as imp;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
use stub as imp;

pub use imp::Capture;

/// 官方 App 写声控帧的间隔。
pub const TICK: Duration = Duration::from_millis(100);

/// 有声：RR+GG+BB ≥ 0x65，三通道都 ≥ 0x67 时灯带不间断换色。
pub const LOUD_RGB: crate::protocol::Rgb = crate::protocol::Rgb::new(0xff, 0xff, 0xff);

/// 音量条还能看见就算有声；只有低过这条才算彻底静音。
pub const SILENCE_RMS: f32 = 0.01;

const THRESHOLD_HIGH: f32 = 0.160;
const THRESHOLD_LOW: f32 = 0.018;

/// 灵敏度 1–100，越大越容易触发。70 ≈ 原先固定门槛 0.065。
pub fn loud_rms(percent: f32) -> f32 {
    let t = ((percent - 1.0) / 99.0).clamp(0.0, 1.0);
    THRESHOLD_HIGH * (1.0 - t) + THRESHOLD_LOW * t
}

/// `loud_rms` 的反函数，门槛越高灵敏度越低。
pub fn percent_from_rms(threshold: f32) -> f32 {
    let span = THRESHOLD_HIGH - THRESHOLD_LOW;
    let t = ((THRESHOLD_HIGH - threshold) / span).clamp(0.0, 1.0);
    (t * 99.0 + 1.0).clamp(1.0, 100.0)
}

/// 近 15 秒响度：门槛埋进底噪就算太灵敏，峰值过不去就算太沉默。
#[derive(Clone, Debug, Default)]
pub struct DynamicSense {
    samples: VecDeque<(Instant, f32)>,
    last_change: Option<Instant>,
}

impl DynamicSense {
    pub const WINDOW: Duration = Duration::from_secs(15);
    const WARMUP: Duration = Duration::from_secs(5);
    const COOLDOWN: Duration = Duration::from_secs(1);
    const STEP: f32 = 5.0;
    const LOUD_CONTENT: f32 = 0.10;
    const SILENT_HOLD: Duration = Duration::from_secs(1);

    pub fn clear(&mut self) {
        self.samples.clear();
        self.last_change = None;
    }

    pub fn push(&mut self, now: Instant, rms: f32) {
        self.samples.push_back((now, rms));
        while let Some(&(at, _)) = self.samples.front() {
            if now.duration_since(at) > Self::WINDOW {
                self.samples.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn suggest(&mut self, now: Instant, current: f32) -> Option<f32> {
        let Some(&(oldest, _)) = self.samples.front() else {
            return None;
        };
        if now.duration_since(oldest) < Self::WARMUP {
            return None;
        }
        if self
            .last_change
            .is_some_and(|at| now.duration_since(at) < Self::COOLDOWN)
        {
            return None;
        }

        if self.recently_silent(now) {
            return None;
        }

        let mut values: Vec<f32> = self.samples.iter().map(|(_, rms)| *rms).collect();
        values.sort_by(|a, b| a.total_cmp(b));
        let floor = percentile(&values, 0.2);
        let peak = percentile(&values, 0.95);
        let threshold = loud_rms(current);

        if peak < SILENCE_RMS * 1.5 {
            return None;
        }

        let target_th = if floor >= threshold && floor < Self::LOUD_CONTENT {
            floor * 1.2
        } else if peak < threshold && peak >= SILENCE_RMS * 3.0 {
            let span = (peak - floor).max(0.0);
            (floor + span * 0.35).min(peak * 0.92)
        } else {
            return None;
        };

        let target = percent_from_rms(target_th);
        let delta = target - current;
        if delta.abs() < 2.0 {
            return None;
        }
        let next = (current + delta.signum() * delta.abs().min(Self::STEP)).clamp(1.0, 100.0);
        if (next - current).abs() < 0.5 {
            return None;
        }
        self.last_change = Some(now);
        Some(next)
    }

    fn recently_silent(&self, now: Instant) -> bool {
        let mut n = 0usize;
        for (at, rms) in self.samples.iter().rev() {
            if now.duration_since(*at) > Self::SILENT_HOLD {
                break;
            }
            n += 1;
            if *rms > SILENCE_RMS {
                return false;
            }
        }
        n > 0
    }
}

fn percentile(sorted: &[f32], p: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let i = ((sorted.len() - 1) as f32 * p).round() as usize;
    sorted[i.min(sorted.len() - 1)]
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sample {
    pub rms: f32,
    pub loud: bool,
}

#[derive(Debug)]
pub enum Error {
    Unsupported,
    Backend(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => write!(f, "这一系统还没有电脑音频抓取"),
            Self::Backend(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for Error {}

pub fn supported() -> bool {
    imp::SUPPORTED
}

pub fn rms_f32(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Gate {
    loud: bool,
}

impl Gate {
    pub fn push(&mut self, rms: f32, threshold: f32) -> bool {
        self.loud = rms >= threshold;
        self.loud
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_is_quiet() {
        assert_eq!(rms_f32(&[0.0, 0.0, 0.0, 0.0]), 0.0);
        let th = loud_rms(70.0);
        let mut gate = Gate::default();
        assert!(!gate.push(0.0, th));
        assert!(!gate.push(0.02, th));
        for _ in 0..8 {
            assert!(!gate.push(0.04, th));
        }
    }

    #[test]
    fn opens_on_first_loud_frame() {
        let mut gate = Gate::default();
        assert!(gate.push(0.12, loud_rms(70.0)));
    }

    #[test]
    fn below_threshold_stays_quiet() {
        let mut gate = Gate::default();
        let th = loud_rms(70.0);
        assert!(!gate.push(0.05, th));
        assert!(!gate.push(th - 0.001, th));
    }

    #[test]
    fn closes_on_first_quiet_frame() {
        let mut gate = Gate::default();
        let th = loud_rms(70.0);
        assert!(gate.push(0.12, th));
        assert!(!gate.push(0.01, th));
    }

    #[test]
    fn louder_percent_means_lower_threshold() {
        assert!(loud_rms(100.0) < loud_rms(70.0));
        assert!(loud_rms(70.0) < loud_rms(1.0));
        assert!((loud_rms(70.0) - 0.061).abs() < 0.01);
    }

    #[test]
    fn percent_roundtrips_loud_rms() {
        for percent in [1.0, 30.0, 70.0, 100.0] {
            let back = percent_from_rms(loud_rms(percent));
            assert!((back - percent).abs() < 0.02, "{percent} -> {back}");
        }
    }

    fn fill(sense: &mut DynamicSense, start: Instant, rms: impl Fn(u32) -> f32, n: u32) {
        for i in 0..n {
            sense.push(start + Duration::from_millis(100 * i as u64), rms(i));
        }
    }

    #[test]
    fn dynamic_waits_for_warmup() {
        let start = Instant::now();
        let mut sense = DynamicSense::default();
        fill(&mut sense, start, |_| 0.04, 20);
        let now = start + Duration::from_millis(1900);
        assert_eq!(sense.suggest(now, 100.0), None);
    }

    #[test]
    fn dynamic_ignores_true_silence() {
        let start = Instant::now();
        let mut sense = DynamicSense::default();
        fill(&mut sense, start, |_| 0.004, 80);
        let now = start + Duration::from_millis(7900);
        assert_eq!(sense.suggest(now, 70.0), None);
    }

    #[test]
    fn dynamic_lowers_sensitivity_when_floor_always_crosses() {
        let start = Instant::now();
        let mut sense = DynamicSense::default();
        fill(&mut sense, start, |_| 0.04, 80);
        let now = start + Duration::from_millis(7900);
        let next = sense.suggest(now, 100.0).expect("too sensitive");
        assert!(next < 100.0, "{next}");
        assert!(loud_rms(next) > loud_rms(100.0));
    }

    #[test]
    fn dynamic_lowers_using_recent_window() {
        let start = Instant::now();
        let mut sense = DynamicSense::default();
        fill(&mut sense, start, |_| 0.004, 80);
        fill(
            &mut sense,
            start + DynamicSense::WINDOW,
            |_| 0.04,
            80,
        );
        let now = start + DynamicSense::WINDOW + Duration::from_millis(7900);
        let next = sense.suggest(now, 100.0).expect("recent floor too hot");
        assert!(next < 100.0, "{next}");
    }

    #[test]
    fn dynamic_does_not_lower_on_loudness_older_than_window() {
        let start = Instant::now();
        let mut sense = DynamicSense::default();
        fill(&mut sense, start, |_| 0.04, 80);
        fill(
            &mut sense,
            start + DynamicSense::WINDOW,
            |_| 0.004,
            80,
        );
        let now = start + DynamicSense::WINDOW + Duration::from_millis(7900);
        assert_eq!(sense.suggest(now, 100.0), None);
    }

    #[test]
    fn dynamic_raises_sensitivity_when_peaks_miss_gate() {
        let start = Instant::now();
        let mut sense = DynamicSense::default();
        fill(
            &mut sense,
            start,
            |i| {
                if i % 10 == 0 {
                    0.08
                } else {
                    0.004
                }
            },
            80,
        );
        let now = start + Duration::from_millis(7900);
        let next = sense.suggest(now, 1.0).expect("too silent");
        assert!(next > 1.0, "{next}");
        assert!(loud_rms(next) < loud_rms(1.0));
    }

    #[test]
    fn dynamic_ignores_when_sound_drops_to_zero() {
        let start = Instant::now();
        let mut sense = DynamicSense::default();
        fill(
            &mut sense,
            start,
            |i| {
                if i % 10 == 0 {
                    0.08
                } else {
                    0.004
                }
            },
            80,
        );
        fill(
            &mut sense,
            start + Duration::from_millis(8000),
            |_| 0.0,
            20,
        );
        let now = start + Duration::from_millis(9900);
        assert_eq!(sense.suggest(now, 1.0), None);
        assert_eq!(sense.suggest(now, 70.0), None);
    }

    #[test]
    fn dynamic_leaves_mixed_audio_alone() {
        let start = Instant::now();
        let mut sense = DynamicSense::default();
        fill(
            &mut sense,
            start,
            |i| {
                if i % 4 == 0 {
                    0.14
                } else {
                    0.02
                }
            },
            80,
        );
        let now = start + Duration::from_millis(7900);
        assert_eq!(sense.suggest(now, 70.0), None);
    }

    #[test]
    fn dynamic_drops_samples_older_than_window() {
        let start = Instant::now();
        let mut sense = DynamicSense::default();
        fill(&mut sense, start, |_| 0.04, 80);
        let later = start + DynamicSense::WINDOW + Duration::from_secs(2);
        fill(&mut sense, later, |_| 0.004, 80);
        let now = later + Duration::from_millis(7900);
        assert_eq!(sense.suggest(now, 70.0), None);
    }
}
