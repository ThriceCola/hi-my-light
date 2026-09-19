use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::lamp::{Front, Rear, RearLook};
use hi_my_light::{Cct, Effect, Level, Rgb};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClosePreference {
    #[default]
    Ask,
    Quit,
    Background,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub last_addr: Option<String>,
    pub last_name: Option<String>,
    #[serde(default)]
    pub close_preference: ClosePreference,
    #[serde(default)]
    pub restore_after_shutdown: bool,
    #[serde(default)]
    pub autostart: bool,
    #[serde(default = "default_true")]
    pub off_on_shutdown: bool,
    #[serde(default)]
    pub front: FrontSnap,
    #[serde(default)]
    pub rear: RearSnap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontSnap {
    #[serde(default = "default_true")]
    pub on: bool,
    #[serde(default = "default_level")]
    pub level: u8,
    #[serde(default = "default_kelvin")]
    pub kelvin: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RearSnap {
    #[serde(default)]
    pub on: bool,
    #[serde(default = "default_level")]
    pub level: u8,
    #[serde(default = "default_rgb")]
    pub rgb: [u8; 3],
    #[serde(default)]
    pub effect: Option<u8>,
    #[serde(default)]
    pub playing: Option<bool>,
    #[serde(default = "default_speed")]
    pub speed: u8,
    #[serde(default)]
    pub audio: bool,
    #[serde(default = "default_audio_sensitivity")]
    pub audio_sensitivity: u8,
    #[serde(default)]
    pub audio_dynamic: bool,
}

fn default_true() -> bool {
    true
}

fn default_level() -> u8 {
    48
}

fn default_kelvin() -> u16 {
    4500
}

fn default_rgb() -> [u8; 3] {
    [0xE0, 0xA8, 0x5C]
}

fn default_speed() -> u8 {
    0x5a
}

fn default_audio_sensitivity() -> u8 {
    70
}

impl Default for FrontSnap {
    fn default() -> Self {
        Self {
            on: true,
            level: default_level(),
            kelvin: default_kelvin(),
        }
    }
}

impl Default for RearSnap {
    fn default() -> Self {
        Self {
            on: false,
            level: default_level(),
            rgb: default_rgb(),
            effect: None,
            playing: None,
            speed: default_speed(),
            audio: false,
            audio_sensitivity: default_audio_sensitivity(),
            audio_dynamic: false,
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self {
            last_addr: None,
            last_name: None,
            close_preference: ClosePreference::Ask,
            restore_after_shutdown: false,
            autostart: false,
            off_on_shutdown: true,
            front: FrontSnap::default(),
            rear: RearSnap::default(),
        }
    }
}

impl Session {
    pub fn path() -> PathBuf {
        let dir = {
            #[cfg(windows)]
            {
                std::env::current_exe()
                    .ok()
                    .and_then(|exe| exe.parent().map(PathBuf::from))
                    .unwrap_or_else(|| PathBuf::from("."))
            }
            #[cfg(not(windows))]
            {
                std::env::var_os("XDG_DATA_HOME")
                    .map(PathBuf::from)
                    .or_else(|| {
                        std::env::var_os("HOME")
                            .map(|home| PathBuf::from(home).join(".local/share"))
                    })
                    .unwrap_or_else(|| PathBuf::from("."))
            }
        };
        #[cfg(windows)]
        {
            dir.join("session.json")
        }
        #[cfg(not(windows))]
        {
            dir.join("hi-my-light").join("session.json")
        }
    }

    pub fn load() -> Self {
        let path = Self::path();
        let Ok(bytes) = fs::read(&path) else {
            return Self::default();
        };
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }

    pub fn front_lamp(&self) -> Front {
        Front::new(
            self.front.on,
            Level::from_byte(self.front.level),
            Cct::from_kelvin(self.front.kelvin),
        )
    }

    pub fn rear_lamp(&self) -> Rear {
        let solid = Rgb::new(self.rear.rgb[0], self.rear.rgb[1], self.rear.rgb[2]);
        let effect = self
            .rear
            .effect
            .and_then(effect_from_byte)
            .unwrap_or(Effect::RainbowFwd);
        let playing = self.rear.playing.unwrap_or(self.rear.effect.is_some());
        let look = if self.rear.audio {
            RearLook::Audio
        } else if playing {
            RearLook::Play(effect)
        } else {
            RearLook::Solid(solid)
        };
        Rear::new(
            self.rear.on,
            Level::from_byte(self.rear.level),
            look,
            Level::from_byte(self.rear.speed),
        )
        .with_memory(solid, effect)
    }
}

impl FrontSnap {
    pub fn from_lamp(front: &Front) -> Self {
        Self {
            on: front.on,
            level: front.level.byte(),
            kelvin: front.cct.kelvin(),
        }
    }
}

impl RearSnap {
    pub fn from_lamp(rear: &Rear) -> Self {
        Self {
            on: rear.on,
            level: rear.level.byte(),
            rgb: [rear.solid.r, rear.solid.g, rear.solid.b],
            effect: Some(rear.effect.byte()),
            playing: Some(matches!(rear.look, RearLook::Play(_))),
            speed: rear.speed.byte(),
            audio: matches!(rear.look, RearLook::Audio),
            audio_sensitivity: default_audio_sensitivity(),
            audio_dynamic: false,
        }
    }
}

fn effect_from_byte(byte: u8) -> Option<Effect> {
    Effect::ALL.into_iter().find(|effect| effect.byte() == byte)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lamp::RearLook;

    #[test]
    fn solid_keeps_last_effect() {
        let mut rear = Rear::new(
            true,
            Level::from_byte(40),
            RearLook::Play(Effect::RgbJump),
            Level::from_byte(0x5a),
        )
        .with_memory(Rgb::new(0x11, 0x22, 0x33), Effect::RgbJump);
        rear.set_rgb(Rgb::new(0xAA, 0xBB, 0xCC));
        let snap = RearSnap::from_lamp(&rear);
        assert_eq!(snap.rgb, [0xAA, 0xBB, 0xCC]);
        assert_eq!(snap.effect, Some(Effect::RgbJump.byte()));
        assert_eq!(snap.playing, Some(false));
        let restored = Session {
            rear: snap,
            ..Session::default()
        }
        .rear_lamp();
        assert_eq!(restored.look, RearLook::Solid(Rgb::new(0xAA, 0xBB, 0xCC)));
        assert_eq!(restored.effect, Effect::RgbJump);
        assert_eq!(restored.solid, Rgb::new(0xAA, 0xBB, 0xCC));
    }

    #[test]
    fn effect_keeps_last_solid() {
        let mut rear = Rear::new(
            true,
            Level::from_byte(40),
            RearLook::Solid(Rgb::new(0x11, 0x22, 0x33)),
            Level::from_byte(0x5a),
        );
        rear.set_effect(Effect::BlueHorse);
        let snap = RearSnap::from_lamp(&rear);
        assert_eq!(snap.rgb, [0x11, 0x22, 0x33]);
        assert_eq!(snap.effect, Some(Effect::BlueHorse.byte()));
        assert_eq!(snap.playing, Some(true));
        let restored = Session {
            rear: snap,
            ..Session::default()
        }
        .rear_lamp();
        assert_eq!(restored.look, RearLook::Play(Effect::BlueHorse));
        assert_eq!(restored.solid, Rgb::new(0x11, 0x22, 0x33));
    }

    #[test]
    fn legacy_effect_without_playing_is_motion() {
        let json = r#"{"on":true,"level":48,"rgb":[1,2,3],"effect":193,"speed":90}"#;
        let snap: RearSnap = serde_json::from_str(json).unwrap();
        let restored = Session {
            rear: snap,
            ..Session::default()
        }
        .rear_lamp();
        assert_eq!(restored.look, RearLook::Play(Effect::RainbowJump));
        assert_eq!(restored.solid, Rgb::new(1, 2, 3));
    }

    #[test]
    fn audio_look_roundtrip() {
        let mut rear = Rear::new(
            true,
            Level::from_byte(40),
            RearLook::Solid(Rgb::new(0x11, 0x22, 0x33)),
            Level::from_byte(0x5a),
        );
        rear.show_audio();
        let snap = RearSnap::from_lamp(&rear);
        assert!(snap.audio);
        assert_eq!(snap.playing, Some(false));
        let restored = Session {
            rear: snap,
            ..Session::default()
        }
        .rear_lamp();
        assert_eq!(restored.look, RearLook::Audio);
        assert_eq!(restored.solid, Rgb::new(0x11, 0x22, 0x33));
    }
}
