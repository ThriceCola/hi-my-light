//! 电脑正在播放的声音（不是麦克风）。
//!
//! 各系统抓法差很多，实现按操作系统分目录、用 `cfg` 选进去。
//! 调用方只看有没有声、有多响。

use std::time::Duration;

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

/// 灵敏度 1–100，越大越容易触发。70 ≈ 原先固定门槛 0.065。
pub fn loud_rms(percent: f32) -> f32 {
    let t = ((percent - 1.0) / 99.0).clamp(0.0, 1.0);
    0.160 * (1.0 - t) + 0.018 * t
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
}
