use super::Frame;

/// 前置色温。WW+CW 始终 = `0x64`。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Cct {
    kelvin: u16,
}

const KNOTS: [(u16, u8); 5] = [
    (3000, 0x64),
    (4000, 0x4b),
    (4500, 0x32),
    (5000, 0x19),
    (6000, 0x00),
];

impl Cct {
    pub const WARM: Self = Self { kelvin: 3000 };
    pub const READ: Self = Self { kelvin: 4500 };
    pub const COOL: Self = Self { kelvin: 6000 };

    pub const MIN_K: u16 = 3000;
    pub const MAX_K: u16 = 6000;

    pub const fn from_kelvin(kelvin: u16) -> Self {
        let kelvin = if kelvin < Self::MIN_K {
            Self::MIN_K
        } else if kelvin > Self::MAX_K {
            Self::MAX_K
        } else {
            kelvin
        };
        Self { kelvin }
    }

    pub const fn kelvin(self) -> u16 {
        self.kelvin
    }

    pub fn mix(self) -> WhiteMix {
        WhiteMix::from_warm(warm_for(self.kelvin))
    }

    /// `7e 07 05 02 WW CW 02 01 ef`
    pub fn frame(self) -> Frame {
        self.mix().frame()
    }
}

/// 暖白份额，冷白 = `0x64 - warm`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WhiteMix {
    warm: u8,
}

impl WhiteMix {
    pub const TOTAL: u8 = 0x64;

    pub const fn from_warm(warm: u8) -> Self {
        Self {
            warm: if warm > Self::TOTAL {
                Self::TOTAL
            } else {
                warm
            },
        }
    }

    pub const fn warm(self) -> u8 {
        self.warm
    }

    pub const fn cool(self) -> u8 {
        Self::TOTAL - self.warm
    }

    pub const fn frame(self) -> Frame {
        Frame::pack([0x07, 0x05, 0x02, self.warm(), self.cool(), 0x02, 0x01])
    }
}

fn warm_for(kelvin: u16) -> u8 {
    for pair in KNOTS.windows(2) {
        let (k0, w0) = pair[0];
        let (k1, w1) = pair[1];
        if kelvin <= k1 {
            let span = (k1 - k0) as f32;
            let t = (kelvin - k0) as f32 / span;
            return (w0 as f32 + (w1 as f32 - w0 as f32) * t).round() as u8;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_presets() {
        assert_eq!(Cct::from_kelvin(3000).frame().hex(), "7e 07 05 02 64 00 02 01 ef");
        assert_eq!(Cct::from_kelvin(4000).frame().hex(), "7e 07 05 02 4b 19 02 01 ef");
        assert_eq!(Cct::from_kelvin(4500).frame().hex(), "7e 07 05 02 32 32 02 01 ef");
        assert_eq!(Cct::from_kelvin(5000).frame().hex(), "7e 07 05 02 19 4b 02 01 ef");
        assert_eq!(Cct::from_kelvin(6000).frame().hex(), "7e 07 05 02 00 64 02 01 ef");
    }

    #[test]
    fn mix_always_sums_to_100() {
        for k in (3000..=6000).step_by(50) {
            let mix = Cct::from_kelvin(k).mix();
            assert_eq!(mix.warm() + mix.cool(), 0x64);
        }
    }
}
