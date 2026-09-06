use super::Frame;

/// 后置纯色。`7e 07 05 03 RR GG BB 10 ef`
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const RED: Self = Self::new(0xff, 0x00, 0x00);
    pub const GREEN: Self = Self::new(0x00, 0xff, 0x00);
    pub const BLUE: Self = Self::new(0x00, 0x00, 0xff);
    pub const CYAN: Self = Self::new(0x00, 0xff, 0xff);
    pub const YELLOW: Self = Self::new(0xff, 0xff, 0x00);
    pub const PURPLE: Self = Self::new(0xc0, 0x00, 0xff);
    pub const WHITE: Self = Self::new(0xff, 0xff, 0xff);

    pub const PRESETS: [(&'static str, Self); 6] = [
        ("赤陶", Self::new(0xE0, 0x7A, 0x5F)),
        ("琥珀", Self::new(0xE0, 0xA8, 0x5C)),
        ("松绿", Self::new(0x4C, 0xAF, 0x8A)),
        ("雾蓝", Self::new(0x6A, 0x9B, 0xC3)),
        ("丁香", Self::new(0xA6, 0x7D, 0xB8)),
        ("白", Self::WHITE),
    ];

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn scale(self, amount: f32) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        Self::new(
            (self.r as f32 * amount).round() as u8,
            (self.g as f32 * amount).round() as u8,
            (self.b as f32 * amount).round() as u8,
        )
    }

    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self::new(
            mix_u8(self.r, other.r, t),
            mix_u8(self.g, other.g, t),
            mix_u8(self.b, other.b, t),
        )
    }

    pub fn hue(self) -> f32 {
        self.hsv().0
    }

    pub fn hsv(self) -> (f32, f32, f32) {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let d = max - min;
        let h = if d < f32::EPSILON {
            0.0
        } else if (max - r).abs() < f32::EPSILON {
            60.0 * ((g - b) / d).rem_euclid(6.0)
        } else if (max - g).abs() < f32::EPSILON {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        let s = if max < f32::EPSILON { 0.0 } else { d / max };
        (h.rem_euclid(360.0), s, max)
    }

    pub fn from_hsv(hue: f32, sat: f32, val: f32) -> Self {
        let s = sat.clamp(0.0, 1.0);
        let v = val.clamp(0.0, 1.0);
        let h = hue.rem_euclid(360.0) / 60.0;
        let c = v * s;
        let x = c * (1.0 - (h % 2.0 - 1.0).abs());
        let m = v - c;
        let (r, g, b) = match h as u8 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        Self::new(to_u8(r + m), to_u8(g + m), to_u8(b + m))
    }

    pub fn from_hue(degrees: f32) -> Self {
        Self::from_hsv(degrees, 1.0, 1.0)
    }

    pub const fn packed(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
    }

    pub const fn frame(self) -> Frame {
        Frame::pack([0x07, 0x05, 0x03, self.r, self.g, self.b, 0x10])
    }
}

fn to_u8(channel: f32) -> u8 {
    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn mix_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_presets() {
        assert_eq!(Rgb::RED.frame().hex(), "7e 07 05 03 ff 00 00 10 ef");
        assert_eq!(Rgb::GREEN.frame().hex(), "7e 07 05 03 00 ff 00 10 ef");
        assert_eq!(Rgb::BLUE.frame().hex(), "7e 07 05 03 00 00 ff 10 ef");
        assert_eq!(Rgb::CYAN.frame().hex(), "7e 07 05 03 00 ff ff 10 ef");
        assert_eq!(Rgb::YELLOW.frame().hex(), "7e 07 05 03 ff ff 00 10 ef");
        assert_eq!(Rgb::WHITE.frame().hex(), "7e 07 05 03 ff ff ff 10 ef");
    }

    #[test]
    fn hue_hits_primaries() {
        assert_eq!(Rgb::from_hue(0.0), Rgb::RED);
        assert_eq!(Rgb::from_hue(120.0), Rgb::GREEN);
        assert_eq!(Rgb::from_hue(240.0), Rgb::BLUE);
    }

    #[test]
    fn hsv_roundtrip_primaries() {
        for color in [Rgb::RED, Rgb::GREEN, Rgb::BLUE, Rgb::WHITE] {
            let (h, s, v) = color.hsv();
            assert_eq!(Rgb::from_hsv(h, s, v), color);
        }
        assert_eq!(Rgb::from_hsv(0.0, 0.0, 0.0), Rgb::new(0, 0, 0));
        let mist = Rgb::from_hsv(210.0, 0.45, 0.80);
        let (h, s, v) = mist.hsv();
        assert!((h - 210.0).abs() < 2.0);
        assert!((s - 0.45).abs() < 0.02);
        assert!((v - 0.80).abs() < 0.02);
    }
}
