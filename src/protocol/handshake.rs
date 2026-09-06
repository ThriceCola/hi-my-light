use super::Frame;

/// `7e 07 ea 02 01 ff ff 00 ef`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Query;

impl Query {
    pub const fn frame(self) -> Frame {
        Frame::pack([0x07, 0xea, 0x02, 0x01, 0xff, 0xff, 0x00])
    }
}

/// App 握手第二帧。HH MM 是本地时分；其后一字节按秒写（抓包未完全确认）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl Clock {
    pub fn now() -> Self {
        let (hour, minute, second) = local_hms();
        Self {
            hour,
            minute,
            second,
        }
    }

    pub fn frame(self) -> Frame {
        Frame::pack([
            0x06,
            0x83,
            self.hour.min(23),
            self.minute.min(59),
            self.second.min(59),
            0x06,
            0x00,
        ])
    }
}

#[cfg(unix)]
fn local_hms() -> (u8, u8, u8) {
    unsafe {
        let mut t: libc::time_t = 0;
        libc::time(&mut t);
        let tm = libc::localtime(&t);
        if tm.is_null() {
            return (0, 0, 0);
        }
        let tm = *tm;
        (tm.tm_hour as u8, tm.tm_min as u8, tm.tm_sec as u8)
    }
}

#[cfg(not(unix))]
fn local_hms() -> (u8, u8, u8) {
    (0, 0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_matches_capture() {
        assert_eq!(Query.frame().hex(), "7e 07 ea 02 01 ff ff 00 ef");
    }

    #[test]
    fn clock_layout() {
        let clock = Clock {
            hour: 0x17,
            minute: 0x2c,
            second: 0x00,
        };
        assert_eq!(clock.frame().hex(), "7e 06 83 17 2c 00 06 00 ef");
    }
}
