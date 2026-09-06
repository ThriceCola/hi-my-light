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
    pub effect: Option<u8>,
    #[serde(default = "default_speed")]
    pub speed: u8,
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
            speed: default_speed(),
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
                std::env::var_os("LOCALAPPDATA")
                    .map(PathBuf::from)
                    .or_else(|| {
                        std::env::var_os("USERPROFILE")
                            .map(|home| PathBuf::from(home).join("AppData").join("Local"))
                    })
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
        dir.join("hi-my-light").join("session.json")
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
        let look = self
            .rear
            .effect
            .and_then(effect_from_byte)
            .map(RearLook::Play)
            .unwrap_or_else(|| {
                RearLook::Solid(Rgb::new(self.rear.rgb[0], self.rear.rgb[1], self.rear.rgb[2]))
            });
        Rear::new(
            self.rear.on,
            Level::from_byte(self.rear.level),
            look,
            Level::from_byte(self.rear.speed),
        )
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
        let (rgb, effect) = match rear.look {
            RearLook::Solid(rgb) => ([rgb.r, rgb.g, rgb.b], None),
            RearLook::Play(effect) => {
                let rgb = rear.rgb();
                ([rgb.r, rgb.g, rgb.b], Some(effect.byte()))
            }
        };
        Self {
            on: rear.on,
            level: rear.level.byte(),
            rgb,
            effect,
            speed: rear.speed.byte(),
        }
    }
}

fn effect_from_byte(byte: u8) -> Option<Effect> {
    Effect::ALL.into_iter().find(|effect| effect.byte() == byte)
}
