use super::{Frame, Level};

/// 后置灯效速度。`7e 07 02 SP ff ff ff 00 ef`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Speed(Level);

impl Speed {
    pub const fn new(level: Level) -> Self {
        Self(level)
    }

    pub const fn level(self) -> Level {
        self.0
    }

    pub const fn frame(self) -> Frame {
        Frame::pack([0x07, 0x02, self.0.byte(), 0xff, 0xff, 0xff, 0x00])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_extrema() {
        assert_eq!(Speed::new(Level::MIN).frame().hex(), "7e 07 02 01 ff ff ff 00 ef");
        assert_eq!(Speed::new(Level::MAX).frame().hex(), "7e 07 02 64 ff ff ff 00 ef");
    }
}
