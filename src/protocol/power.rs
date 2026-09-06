use super::{Channel, Frame};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Switch {
    Off,
    On,
}

impl Switch {
    pub const fn xx(self) -> u8 {
        match self {
            Self::Off => 0x00,
            Self::On => 0xff,
        }
    }

    pub const fn yy(self) -> u8 {
        match self {
            Self::Off => 0x00,
            Self::On => 0x01,
        }
    }

    pub const fn is_on(self) -> bool {
        matches!(self, Self::On)
    }
}

/// `7e 07 04 XX CH YY 02 01 ef`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Power {
    pub channel: Channel,
    pub switch: Switch,
}

impl Power {
    pub const fn on(channel: Channel) -> Self {
        Self {
            channel,
            switch: Switch::On,
        }
    }

    pub const fn off(channel: Channel) -> Self {
        Self {
            channel,
            switch: Switch::Off,
        }
    }

    pub const fn frame(self) -> Frame {
        Frame::pack([
            0x07,
            0x04,
            self.switch.xx(),
            self.channel.byte(),
            self.switch.yy(),
            0x02,
            0x01,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_front_and_rear() {
        assert_eq!(
            Power::off(Channel::Front).frame().hex(),
            "7e 07 04 00 05 00 02 01 ef"
        );
        assert_eq!(
            Power::on(Channel::Front).frame().hex(),
            "7e 07 04 ff 05 01 02 01 ef"
        );
        assert_eq!(
            Power::off(Channel::Rear).frame().hex(),
            "7e 07 04 00 01 00 02 01 ef"
        );
        assert_eq!(
            Power::on(Channel::Rear).frame().hex(),
            "7e 07 04 ff 01 01 02 01 ef"
        );
    }
}
