//! 声控帧 `0x20` 的 RGB 不是灯珠颜色。灯带自己跑七彩。
//! 三字节之和 ≥ 0x65 才跑；白帧是其中「不间断」那种。
//! 换一格 RGB 不会在灯上留下另一种颜色。安静时停写，或写一帧全零。

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::protocol::Rgb;

use super::loud_rms;

/// 抓包里把灯带上旧颜色挤出去大约要这么多格。
pub const STRIP_CELLS: u8 = 20;

const STEP: Duration = Duration::from_millis(80);
const HYSTERESIS: u8 = 1;
const SHRINK_DELAY: Duration = Duration::from_millis(140);

const L_FLOOR: f32 = 0.10;
const K_BASE: f32 = 0.50;
const K_DOWN: f32 = 0.38;
const LOW_TH: f32 = 0.30;
const SILENCE_NORM: f32 = 0.08;
const PLAY_ON: f32 = 0.02;
const PLAY_OFF: f32 = 0.012;
const PLAY_HOLD: Duration = Duration::from_millis(700);

const REFRACT_DOWN: Duration = Duration::from_millis(150);
const MIN_EVENT_GAP: Duration = Duration::from_millis(450);

const SLOW_TAU: f32 = 0.25;
const NORM_WINDOW: usize = 64;
const NORM_WARMUP: usize = 12;
const FIXED_SCALE: f32 = 0.25;

const ATTACK: Duration = Duration::from_millis(40);
const HOLD_DOWN: Duration = Duration::from_millis(480);
const DECAY_DOWN: Duration = Duration::from_millis(90);

/// 不再用于声控。`0x20` 不会按返回值上色。
pub fn beat_color(solid: Rgb) -> Rgb {
    const FALLBACK: Rgb = Rgb::new(0xE0, 0x7A, 0x5F);
    if solid == Rgb::BLACK || chases_on_device(solid) {
        FALLBACK
    } else {
        solid
    }
}

fn chases_on_device(color: Rgb) -> bool {
    color.r >= 0x67 && color.g >= 0x67 && color.b >= 0x67
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StripCmd {
    /// `0x10` 黑，整条灭。
    Clear,
    /// `0x20` 推进一格。
    Scroll(Rgb),
}

#[derive(Clone, Debug)]
struct Pulse {
    from: f32,
    peak: f32,
    started: Instant,
    hold: Duration,
    decay: Duration,
}

impl Pulse {
    fn height(&self, now: Instant) -> f32 {
        let env = envelope(
            now.saturating_duration_since(self.started),
            ATTACK,
            self.hold,
            self.decay,
        );
        if env <= 0.0 {
            0.0
        } else {
            self.from + (self.peak - self.from) * env
        }
    }
}

fn envelope(t: Duration, attack: Duration, hold: Duration, decay: Duration) -> f32 {
    if t < attack {
        let a = attack.as_secs_f32().max(0.001);
        return (t.as_secs_f32() / a).clamp(0.0, 1.0);
    }
    let after = t.saturating_sub(attack);
    if after < hold {
        return 1.0;
    }
    let into = after.saturating_sub(hold);
    if into >= decay {
        return 0.0;
    }
    let d = decay.as_secs_f32().max(0.001);
    1.0 - (into.as_secs_f32() / d).clamp(0.0, 1.0)
}

/// RMS 慢包络 + 节拍事件 → 灯带长度，范围 0..=1。
#[derive(Clone, Debug)]
pub struct BeatLight {
    rms_slow: f32,
    base: f32,
    last_obs: Option<Instant>,
    fast_hist: VecDeque<f32>,
    norm_lo: f32,
    norm_hi: f32,
    ref_down: Option<Instant>,
    last_event: Option<Instant>,
    playing: bool,
    quiet_since: Option<Instant>,
    pulse: Option<Pulse>,
}

impl Default for BeatLight {
    fn default() -> Self {
        Self {
            rms_slow: 0.0,
            base: 0.0,
            last_obs: None,
            fast_hist: VecDeque::new(),
            norm_lo: 0.0,
            norm_hi: FIXED_SCALE,
            ref_down: None,
            last_event: None,
            playing: false,
            quiet_since: None,
            pulse: None,
        }
    }
}

impl BeatLight {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// 每块新音频调用一次。`sensitivity` 是 1..=100，越大越容易触发。
    /// 只接受重拍。弱拍不传入、也不产生任何灯效。
    pub fn observe(&mut self, now: Instant, rms_fast: f32, downbeat: bool, sensitivity: f32) {
        if !rms_fast.is_finite() || rms_fast < 0.0 {
            return;
        }
        let dt = self
            .last_obs
            .map(|at| {
                now.saturating_duration_since(at)
                    .as_secs_f32()
                    .clamp(0.02, 0.5)
            })
            .unwrap_or(0.1);
        self.last_obs = Some(now);
        let alpha = 1.0 - (-dt / SLOW_TAU).exp();
        self.rms_slow += alpha * (rms_fast - self.rms_slow);

        self.fast_hist.push_back(rms_fast);
        while self.fast_hist.len() > NORM_WINDOW {
            self.fast_hist.pop_front();
        }
        self.fit_norm();

        let fast = self.norm(rms_fast);
        let slow = self.norm(self.rms_slow);
        self.base = if slow < SILENCE_NORM && fast < SILENCE_NORM {
            0.0
        } else {
            (L_FLOOR + K_BASE * slow).clamp(L_FLOOR, 0.60)
        };

        let low = thresh(LOW_TH, sensitivity);
        self.track_playing(now, downbeat && fast > low);
        let rate_ok = self
            .last_event
            .is_none_or(|at| now.saturating_duration_since(at) >= MIN_EVENT_GAP);

        if downbeat && fast > low && rate_ok && refractory(now, self.ref_down) {
            let peak = (self.base + K_DOWN * fast).clamp(self.base, 0.98);
            self.fire(now, peak, HOLD_DOWN, DECAY_DOWN);
            self.ref_down = Some(now + REFRACT_DOWN);
        }
    }

    pub fn playing(&self) -> bool {
        self.playing
    }

    fn track_playing(&mut self, now: Instant, downbeat: bool) {
        if downbeat || self.rms_slow >= PLAY_ON {
            self.playing = true;
            self.quiet_since = None;
            return;
        }
        if self.rms_slow >= PLAY_OFF {
            return;
        }
        let since = *self.quiet_since.get_or_insert(now);
        if now.saturating_duration_since(since) >= PLAY_HOLD {
            self.playing = false;
        }
    }

    pub fn length(&self, now: Instant) -> f32 {
        let pulse = self.pulse.as_ref().map(|p| p.height(now)).unwrap_or(0.0);
        self.base.max(pulse).clamp(0.0, 1.0)
    }

    /// 只有重拍的保持段。弱拍不会进来。
    pub fn accent(&self, now: Instant) -> bool {
        self.pulse
            .as_ref()
            .is_some_and(|pulse| now.saturating_duration_since(pulse.started) < ATTACK + pulse.hold)
    }

    fn fire(&mut self, now: Instant, peak: f32, hold: Duration, decay: Duration) {
        self.last_event = Some(now);
        self.pulse = Some(Pulse {
            from: self.base,
            peak,
            started: now,
            hold,
            decay,
        });
    }

    fn fit_norm(&mut self) {
        if self.fast_hist.len() < NORM_WARMUP {
            self.norm_lo = 0.0;
            self.norm_hi = FIXED_SCALE;
            return;
        }
        let mut values: Vec<f32> = self.fast_hist.iter().copied().collect();
        values.sort_by(|a, b| a.total_cmp(b));
        let lo = percentile(&values, 0.05);
        let hi = percentile(&values, 0.95);
        if hi - lo < 1.0e-3 {
            // 整段几乎一样响时，百分位会把稳态音乐归一成 0。改回固定刻度。
            self.norm_lo = 0.0;
            self.norm_hi = FIXED_SCALE;
        } else {
            self.norm_lo = lo;
            self.norm_hi = hi;
        }
    }

    fn norm(&self, rms: f32) -> f32 {
        let span = (self.norm_hi - self.norm_lo).max(1.0e-4);
        ((rms - self.norm_lo) / span).clamp(0.0, 1.0)
    }
}

fn thresh(base: f32, sensitivity: f32) -> f32 {
    let ratio = loud_rms(sensitivity) / loud_rms(70.0);
    (base * ratio).clamp(0.02, 0.98)
}

fn refractory(now: Instant, until: Option<Instant>) -> bool {
    until.is_none_or(|at| now >= at)
}

fn percentile(sorted: &[f32], p: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let i = ((sorted.len() - 1) as f32 * p).round() as usize;
    sorted[i.min(sorted.len() - 1)]
}

fn cells_from_length(length: f32) -> u8 {
    if !length.is_finite() || length <= 0.0 {
        return 0;
    }
    let n = (length.clamp(0.0, 1.0) * f32::from(STRIP_CELLS)).round() as u8;
    n.min(STRIP_CELLS)
}

/// 把目标长度变成灯带帧。稳定时不发帧，画面停住。
#[derive(Clone, Debug)]
pub struct StripWriter {
    lit: u8,
    painting: Option<u8>,
    low_since: Option<Instant>,
    last: Option<Instant>,
}

impl Default for StripWriter {
    fn default() -> Self {
        Self {
            lit: 0,
            painting: None,
            low_since: None,
            last: None,
        }
    }
}

impl StripWriter {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn lit(&self) -> u8 {
        self.lit
    }

    pub fn step(&mut self, now: Instant, length: f32, color: Rgb) -> Option<StripCmd> {
        let want = cells_from_length(length);
        if let Some(target) = self.painting {
            let target = target.max(want);
            self.painting = Some(target);
            if self.lit >= target {
                self.painting = None;
                self.low_since = None;
                return None;
            }
            return self.push_color(now, color);
        }

        if want > self.lit {
            self.low_since = None;
            return self.push_color(now, color);
        }

        if self.lit > want.saturating_add(HYSTERESIS) {
            let since = *self.low_since.get_or_insert(now);
            if now.saturating_duration_since(since) >= SHRINK_DELAY && self.due(now) {
                self.painting = Some(want);
                self.lit = 0;
                self.low_since = None;
                self.last = Some(now);
                return Some(StripCmd::Clear);
            }
            return None;
        }

        self.low_since = None;
        None
    }

    fn due(&self, now: Instant) -> bool {
        self.last
            .is_none_or(|at| now.saturating_duration_since(at) >= STEP)
    }

    fn push_color(&mut self, now: Instant, color: Rgb) -> Option<StripCmd> {
        if !self.due(now) || self.lit >= STRIP_CELLS {
            return None;
        }
        self.lit = self.lit.saturating_add(1);
        self.last = Some(now);
        Some(StripCmd::Scroll(color))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ms: u64) -> Instant {
        Instant::now() + Duration::from_millis(ms)
    }

    fn warm(light: &mut BeatLight, start: Instant, rms: f32, n: u32) {
        for i in 0..n {
            light.observe(
                start + Duration::from_millis(100 * u64::from(i)),
                rms,
                false,
                70.0,
            );
        }
    }

    #[test]
    fn silence_stays_dark() {
        let mut light = BeatLight::default();
        let start = Instant::now();
        warm(&mut light, start, 0.002, 20);
        let now = start + Duration::from_millis(2000);
        light.observe(now, 0.002, true, 70.0);
        assert_eq!(light.length(now), 0.0);
    }

    #[test]
    fn steady_music_sits_in_the_base_band() {
        let mut light = BeatLight::default();
        let start = Instant::now();
        warm(&mut light, start, 0.10, 20);
        let now = start + Duration::from_millis(2000);
        let length = light.length(now);
        assert!((0.10..=0.60).contains(&length), "{length}");
    }

    #[test]
    fn downbeat_extends_and_then_falls_back() {
        let mut light = BeatLight::default();
        let start = Instant::now();
        warm(&mut light, start, 0.12, 16);
        let t = start + Duration::from_millis(1600);
        let base = light.length(t);
        light.observe(t, 0.12, true, 70.0);
        let held = light.length(t + Duration::from_millis(80));
        assert!(held > base + 0.08, "base {base} held {held}");
        let later = light.length(t + Duration::from_millis(900));
        assert!(later < held - 0.05, "held {held} later {later}");
    }

    #[test]
    fn second_downbeat_inside_the_gap_does_not_restrike() {
        let mut light = BeatLight::default();
        let start = Instant::now();
        warm(&mut light, start, 0.10, 16);
        let t = start + Duration::from_millis(1600);
        light.observe(t, 0.10, true, 70.0);
        let first = light.length(t + Duration::from_millis(60));
        light.observe(t + Duration::from_millis(80), 0.24, true, 100.0);
        let second = light.length(t + Duration::from_millis(100));
        assert!(
            (second - first).abs() < 0.02,
            "first {first} second {second}"
        );
    }

    #[test]
    fn weak_beat_does_not_extend() {
        let mut light = BeatLight::default();
        let start = Instant::now();
        warm(&mut light, start, 0.12, 16);
        let t = start + Duration::from_millis(1600);
        light.observe(t, 0.22, false, 70.0);
        assert!(!light.accent(t + Duration::from_millis(80)));
        assert!(light.playing());
    }

    #[test]
    fn loud_spike_without_downbeat_does_not_accent() {
        let mut light = BeatLight::default();
        let start = Instant::now();
        warm(&mut light, start, 0.04, 16);
        let t = start + Duration::from_millis(1600);
        assert!(light.playing());
        light.observe(t, 0.22, false, 70.0);
        assert!(!light.accent(t + Duration::from_millis(80)));
        assert!(light.playing());
    }

    #[test]
    fn gap_between_beats_does_not_stop_the_roll() {
        let mut light = BeatLight::default();
        let start = Instant::now();
        warm(&mut light, start, 0.12, 16);
        let t = start + Duration::from_millis(1600);
        assert!(light.playing());
        for i in 0..4 {
            light.observe(t + Duration::from_millis(100 * i), 0.002, false, 70.0);
        }
        assert!(light.playing());
        assert!(!light.accent(t + Duration::from_millis(350)));
        for i in 4..20 {
            light.observe(t + Duration::from_millis(100 * i), 0.002, false, 70.0);
        }
        assert!(!light.playing());
    }

    #[test]
    fn white_is_not_used_as_a_beat_color() {
        assert_ne!(beat_color(Rgb::WHITE), Rgb::WHITE);
        assert!(!chases_on_device(beat_color(Rgb::WHITE)));
        let ink = Rgb::new(0xE0, 0x30, 0x20);
        assert_eq!(beat_color(ink), ink);
    }

    #[test]
    fn strip_grows_one_cell_per_step_then_holds() {
        let mut strip = StripWriter::default();
        let color = Rgb::RED;
        let t0 = at(0);
        assert_eq!(strip.step(t0, 0.15, color), Some(StripCmd::Scroll(color)));
        assert_eq!(
            strip.step(t0 + Duration::from_millis(40), 0.15, color),
            None
        );
        assert_eq!(
            strip.step(t0 + Duration::from_millis(80), 0.15, color),
            Some(StripCmd::Scroll(color))
        );
        assert_eq!(
            strip.step(t0 + Duration::from_millis(160), 0.15, color),
            Some(StripCmd::Scroll(color))
        );
        assert_eq!(strip.lit(), 3);
        assert_eq!(
            strip.step(t0 + Duration::from_millis(240), 0.15, color),
            None
        );
    }

    #[test]
    fn strip_shrinks_by_clearing_then_repainting() {
        let mut strip = StripWriter::default();
        let color = Rgb::RED;
        let mut t = at(0);
        for _ in 0..8 {
            assert!(strip.step(t, 0.40, color).is_some());
            t += STEP;
        }
        assert_eq!(strip.lit(), 8);
        assert_eq!(strip.step(t, 0.10, color), None);
        t += SHRINK_DELAY;
        assert_eq!(strip.step(t, 0.10, color), Some(StripCmd::Clear));
        assert_eq!(strip.lit(), 0);
        t += STEP;
        assert_eq!(strip.step(t, 0.10, color), Some(StripCmd::Scroll(color)));
        t += STEP;
        assert_eq!(strip.step(t, 0.10, color), Some(StripCmd::Scroll(color)));
        assert_eq!(strip.lit(), 2);
    }
}
