//! 响度闪烁和节拍闪烁互斥。
//!
//! 两个窗口分开算：一段时间内暂缓占多少，或正在播放占多少，过了比例就改由节拍写灯。
//! 窗口还没铺满时不切换。纯静音不算暂缓里的「有声音」。

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::{SILENCE_RMS, TICK};

const KEEP: Duration = Duration::from_secs(30);
const FLASH: Duration = Duration::from_millis(300);
const MIN_GAP: Duration = Duration::from_millis(320);
const AUDIBLE_GAP: Duration = Duration::from_millis(800);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeatRule {
    pub quiet_secs: f32,
    pub quiet_pct: f32,
    pub loud_secs: f32,
    pub loud_pct: f32,
    /// 打开后不再看暂缓/正在播放的比例，只要节拍开关开着就按拍闪。
    pub always: bool,
}

impl Default for BeatRule {
    fn default() -> Self {
        Self {
            quiet_secs: 7.0,
            quiet_pct: 100.0,
            loud_secs: 7.0,
            loud_pct: 45.0,
            always: false,
        }
    }
}

impl BeatRule {
    pub fn clamped(self) -> Self {
        Self {
            quiet_secs: self.quiet_secs.clamp(1.0, 30.0),
            quiet_pct: self.quiet_pct.clamp(0.0, 100.0),
            loud_secs: self.loud_secs.clamp(1.0, 30.0),
            loud_pct: self.loud_pct.clamp(0.0, 100.0),
            always: self.always,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drive {
    /// 这一拍交给原来的 RMS。
    Rms,
    /// 写白帧，七彩继续跑。RGB 不是灯珠颜色。
    On,
    /// 写一帧全零，停。
    Off,
    Idle,
}

#[derive(Clone, Copy, Debug)]
struct Mark {
    at: Instant,
    loud: bool,
    audible: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Rhythm {
    samples: VecDeque<Mark>,
    audible_since: Option<Instant>,
    last_audible: Option<Instant>,
    beat_mode: bool,
    flash_until: Option<Instant>,
    need_off: bool,
    last_emit: Option<Instant>,
    last_beat: Option<Instant>,
}

impl Rhythm {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// 每一块新音频记一次。`loud` 是 RMS 门限的结果。
    pub fn note(&mut self, now: Instant, rms: f32, loud: bool) {
        let audible = rms > SILENCE_RMS;
        if audible {
            self.last_audible = Some(now);
            if self.audible_since.is_none() {
                self.audible_since = Some(now);
            }
        } else if self
            .last_audible
            .is_some_and(|at| now.saturating_duration_since(at) > AUDIBLE_GAP)
        {
            self.audible_since = None;
        }
        self.samples.push_back(Mark {
            at: now,
            loud,
            audible,
        });
        self.prune(now);
    }

    pub fn wants_beat(&self, now: Instant, rule: BeatRule) -> bool {
        let rule = rule.clamped();
        rule.always || self.quiet_hit(now, rule) || self.loud_hit(now, rule)
    }

    pub fn lit(&self, now: Instant) -> bool {
        self.beat_mode && self.flash_until.is_some_and(|at| now < at)
    }

    /// `beat` 为真表示这一块里 BeatNet 有拍（含重拍和弱拍）。
    pub fn drive(&mut self, now: Instant, beat: bool, rule: BeatRule) -> Drive {
        self.prune(now);
        let want = self.wants_beat(now, rule);
        if want != self.beat_mode {
            self.beat_mode = want;
            self.flash_until = None;
            self.need_off = false;
            self.last_emit = None;
            if want {
                return Drive::Off;
            }
            return Drive::Rms;
        }
        if !want {
            return Drive::Rms;
        }
        let flashing = self.flash_until.is_some_and(|at| now < at);
        if !flashing && self.need_off {
            self.need_off = false;
            self.last_emit = Some(now);
            if beat {
                self.arm(now);
            }
            return Drive::Off;
        }
        if beat {
            self.arm(now);
        }
        let flashing = self.flash_until.is_some_and(|at| now < at);
        if flashing && self.due(now) {
            self.last_emit = Some(now);
            return Drive::On;
        }
        Drive::Idle
    }

    fn arm(&mut self, now: Instant) {
        if self
            .last_beat
            .is_some_and(|at| now.saturating_duration_since(at) < MIN_GAP)
        {
            return;
        }
        self.last_beat = Some(now);
        self.flash_until = Some(now + FLASH);
        self.need_off = true;
        self.last_emit = None;
    }

    fn due(&self, now: Instant) -> bool {
        self.last_emit
            .is_none_or(|at| now.saturating_duration_since(at) >= TICK)
    }

    fn quiet_hit(&self, now: Instant, rule: BeatRule) -> bool {
        let Some(stat) = self.stat(now, rule.quiet_secs) else {
            return false;
        };
        if stat.audible == 0 {
            return false;
        }
        stat.quiet_pct >= rule.quiet_pct
    }

    fn loud_hit(&self, now: Instant, rule: BeatRule) -> bool {
        let Some(stat) = self.stat(now, rule.loud_secs) else {
            return false;
        };
        stat.loud_pct >= rule.loud_pct
    }

    fn stat(&self, now: Instant, secs: f32) -> Option<Stat> {
        let window = Duration::from_secs_f32(secs);
        let mut total = 0usize;
        let mut loud = 0usize;
        let mut audible = 0usize;
        let mut oldest: Option<Instant> = None;
        for sample in &self.samples {
            if now.saturating_duration_since(sample.at) > window {
                continue;
            }
            oldest = Some(oldest.map_or(sample.at, |at| at.min(sample.at)));
            total += 1;
            if sample.loud {
                loud += 1;
            }
            if sample.audible {
                audible += 1;
            }
        }
        let oldest = oldest?;
        if now.saturating_duration_since(oldest) + Duration::from_millis(200) < window {
            return None;
        }
        if total == 0 {
            return None;
        }
        let scale = 100.0 / total as f32;
        Some(Stat {
            loud_pct: loud as f32 * scale,
            quiet_pct: (total - loud) as f32 * scale,
            audible,
        })
    }

    fn prune(&mut self, now: Instant) {
        while self
            .samples
            .front()
            .is_some_and(|sample| now.saturating_duration_since(sample.at) > KEEP)
        {
            self.samples.pop_front();
        }
    }
}

struct Stat {
    loud_pct: f32,
    quiet_pct: f32,
    audible: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ms: u64) -> Instant {
        Instant::now() + Duration::from_millis(ms)
    }

    fn hold(rhythm: &mut Rhythm, start: Instant, ms: u64, loud: bool, rms: f32) {
        let mut t = 0;
        while t <= ms {
            rhythm.note(start + Duration::from_millis(t), rms, loud);
            t += 100;
        }
    }

    #[test]
    fn short_climax_stays_on_rms() {
        let mut rhythm = Rhythm::default();
        let start = at(0);
        let rule = BeatRule::default();
        hold(&mut rhythm, start, 2000, true, 0.12);
        let now = start + Duration::from_millis(2000);
        assert!(!rhythm.wants_beat(now, rule));
        assert_eq!(rhythm.drive(now, false, rule), Drive::Rms);
    }

    #[test]
    fn sustained_loud_switches_to_beat() {
        let mut rhythm = Rhythm::default();
        let start = at(0);
        let rule = BeatRule::default();
        hold(&mut rhythm, start, 7200, true, 0.12);
        let now = start + Duration::from_millis(7200);
        assert!(rhythm.wants_beat(now, rule));
        assert_eq!(rhythm.drive(now, false, rule), Drive::Off);
        assert_eq!(
            rhythm.drive(now + Duration::from_millis(40), true, rule),
            Drive::On
        );
        assert_eq!(
            rhythm.drive(now + Duration::from_millis(400), false, rule),
            Drive::Off
        );
        assert_eq!(
            rhythm.drive(now + Duration::from_millis(500), false, rule),
            Drive::Idle
        );
    }

    #[test]
    fn quiet_music_for_seven_seconds_uses_beat() {
        let mut rhythm = Rhythm::default();
        let start = at(0);
        let rule = BeatRule::default();
        hold(&mut rhythm, start, 7200, false, 0.03);
        let now = start + Duration::from_millis(7200);
        assert!(rhythm.wants_beat(now, rule));
        assert_eq!(rhythm.drive(now, true, rule), Drive::Off);
    }

    #[test]
    fn silence_does_not_invent_beats() {
        let mut rhythm = Rhythm::default();
        let start = at(0);
        let rule = BeatRule::default();
        hold(&mut rhythm, start, 7200, false, 0.001);
        let now = start + Duration::from_millis(7200);
        assert!(!rhythm.wants_beat(now, rule));
        assert_eq!(rhythm.drive(now, true, rule), Drive::Rms);
    }

    #[test]
    fn lower_loud_percent_catches_a_shorter_stretch() {
        let mut rhythm = Rhythm::default();
        let start = at(0);
        let rule = BeatRule {
            loud_secs: 2.0,
            loud_pct: 80.0,
            ..BeatRule::default()
        };
        hold(&mut rhythm, start, 2000, true, 0.12);
        let now = start + Duration::from_millis(2000);
        assert!(rhythm.wants_beat(now, rule));
        assert!(!rhythm.wants_beat(now, BeatRule::default()));
    }

    #[test]
    fn rms_and_beat_are_not_active_together() {
        let mut rhythm = Rhythm::default();
        let start = at(0);
        let rule = BeatRule::default();
        hold(&mut rhythm, start, 1000, true, 0.12);
        let now = start + Duration::from_millis(1000);
        assert_eq!(rhythm.drive(now, true, rule), Drive::Rms);
        assert!(!rhythm.lit(now));
    }

    #[test]
    fn always_uses_beat_without_waiting() {
        let mut rhythm = Rhythm::default();
        let start = at(0);
        let rule = BeatRule {
            always: true,
            ..BeatRule::default()
        };
        hold(&mut rhythm, start, 200, true, 0.12);
        let now = start + Duration::from_millis(200);
        assert!(rhythm.wants_beat(now, rule));
        assert_eq!(rhythm.drive(now, false, rule), Drive::Off);
    }
}
