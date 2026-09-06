use std::fmt;

/// 9 字节写命令：`7e … ef`，落到 fff3。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame([u8; 9]);

impl Frame {
    pub const HEAD: u8 = 0x7e;
    pub const TAIL: u8 = 0xef;

    pub const fn pack(payload: [u8; 7]) -> Self {
        Self([
            Self::HEAD,
            payload[0],
            payload[1],
            payload[2],
            payload[3],
            payload[4],
            payload[5],
            payload[6],
            Self::TAIL,
        ])
    }

    pub const fn bytes(self) -> [u8; 9] {
        self.0
    }

    pub fn hex(self) -> String {
        self.to_string()
    }
}

impl AsRef<[u8]> for Frame {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, b) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_head_and_tail() {
        let frame = Frame::pack([0x04, 0x01, 0x64, 0x05, 0xff, 0x02, 0x01]);
        assert_eq!(frame.bytes()[0], 0x7e);
        assert_eq!(frame.bytes()[8], 0xef);
        assert_eq!(frame.hex(), "7e 04 01 64 05 ff 02 01 ef");
    }
}
