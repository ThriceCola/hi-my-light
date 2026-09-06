/// 灯通道。协议里写在亮度/电源帧的第五字节。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Channel {
    Rear,
    Front,
}

impl Channel {
    pub const fn byte(self) -> u8 {
        match self {
            Self::Rear => 0x01,
            Self::Front => 0x05,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_match_capture() {
        assert_eq!(Channel::Rear.byte(), 0x01);
        assert_eq!(Channel::Front.byte(), 0x05);
    }
}
