use super::{Channel, Frame, Level};

/// `7e 04 01 LV CH ff 02 01 ef`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Brightness {
    pub channel: Channel,
    pub level: Level,
}

impl Brightness {
    pub const fn new(channel: Channel, level: Level) -> Self {
        Self { channel, level }
    }

    pub const fn frame(self) -> Frame {
        Frame::pack([
            0x04,
            0x01,
            self.level.byte(),
            self.channel.byte(),
            0xff,
            0x02,
            0x01,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_extrema() {
        assert_eq!(
            Brightness::new(Channel::Front, Level::MIN).frame().hex(),
            "7e 04 01 01 05 ff 02 01 ef"
        );
        assert_eq!(
            Brightness::new(Channel::Front, Level::MAX).frame().hex(),
            "7e 04 01 64 05 ff 02 01 ef"
        );
        assert_eq!(
            Brightness::new(Channel::Rear, Level::MIN).frame().hex(),
            "7e 04 01 01 01 ff 02 01 ef"
        );
        assert_eq!(
            Brightness::new(Channel::Rear, Level::MAX).frame().hex(),
            "7e 04 01 64 01 ff 02 01 ef"
        );
    }
}
