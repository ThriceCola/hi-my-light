/// 亮度 / 速度：协议 `0x01..=0x64`（约 1%..=100%）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Level(u8);

impl Level {
    pub const MIN: Self = Self(1);
    pub const MAX: Self = Self(0x64);

    pub const fn from_byte(byte: u8) -> Self {
        Self(if byte < Self::MIN.0 {
            Self::MIN.0
        } else if byte > Self::MAX.0 {
            Self::MAX.0
        } else {
            byte
        })
    }

    pub fn from_percent(percent: f32) -> Self {
        let rounded = percent.round();
        if !rounded.is_finite() {
            return Self::MIN;
        }
        Self::from_byte(rounded as u8)
    }

    pub const fn byte(self) -> u8 {
        self.0
    }

    pub const fn percent(self) -> f32 {
        self.0 as f32
    }
}

impl Default for Level {
    fn default() -> Self {
        Self(48)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_to_protocol_range() {
        assert_eq!(Level::from_percent(0.0).byte(), 0x01);
        assert_eq!(Level::from_percent(100.0).byte(), 0x64);
        assert_eq!(Level::from_percent(101.0).byte(), 0x64);
        assert_eq!(Level::from_percent(f32::NAN).byte(), 0x01);
    }
}
