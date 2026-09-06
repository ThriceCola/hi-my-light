use hi_my_light::{Cct, Level};

#[derive(Clone, Copy)]
pub struct Scene {
    pub id: &'static str,
    pub name: &'static str,
    pub level: Level,
    pub cct: Cct,
}

pub const SCENES: [Scene; 3] = [
    Scene {
        id: "night",
        name: "夜航",
        level: Level::from_byte(16),
        cct: Cct::WARM,
    },
    Scene {
        id: "read",
        name: "阅读",
        level: Level::from_byte(62),
        cct: Cct::READ,
    },
    Scene {
        id: "focus",
        name: "专注",
        level: Level::from_byte(88),
        cct: Cct::from_kelvin(5000),
    },
];
