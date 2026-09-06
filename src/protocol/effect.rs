use super::Frame;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EffectGroup {
    Rainbow,
    Jump,
    Horse,
    Follow,
    Drift,
    Brush,
}

impl EffectGroup {
    pub const ALL: [Self; 6] = [
        Self::Rainbow,
        Self::Jump,
        Self::Horse,
        Self::Follow,
        Self::Drift,
        Self::Brush,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Rainbow => "幻彩",
            Self::Jump => "跳变",
            Self::Horse => "跑马",
            Self::Follow => "追光",
            Self::Drift => "飘动",
            Self::Brush => "刷色",
        }
    }
}

/// 「基础」页灯效。第五字节固定 `06`。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Effect {
    AutoLoop = 0x00,
    RainbowFwd = 0x01,
    RainbowRev = 0x02,
    RainbowEnergy = 0xd4,
    RainbowJump = 0xc1,
    RgbJump = 0xc2,
    YcmJump = 0xc3,
    RainbowStrobe = 0xc4,
    RgbStrobe = 0xc5,
    YcmStrobe = 0xc6,
    SevenFade = 0xc7,
    RedYellowFade = 0xc8,
    RedPurpleFade = 0xc9,
    GreenCyanFade = 0xca,
    GreenYellowFade = 0xcb,
    BluePurpleFade = 0xcc,
    RedHorse = 0xcd,
    GreenHorse = 0xce,
    BlueHorse = 0xcf,
    YellowHorse = 0xd0,
    CyanHorse = 0xd1,
    PurpleHorse = 0xd2,
    WhiteHorse = 0xd3,
    RainbowFollowFwd = 0x4d,
    RainbowFollowRev = 0x4e,
    RgbFollowFwd = 0x4f,
    RgbFollowRev = 0x50,
    YcmFollowFwd = 0x51,
    YcmFollowRev = 0x52,
    RainbowDriftFwd = 0x53,
    RainbowDriftRev = 0x54,
    RgbDriftFwd = 0x55,
    RgbDriftRev = 0x56,
    YcmDriftFwd = 0x57,
    YcmDriftRev = 0x58,
    RainbowBrushFwd = 0xb5,
    RainbowBrushRev = 0xb6,
    RgbBrushFwd = 0xb7,
    RgbBrushRev = 0xb8,
    YcmBrushFwd = 0xb9,
    YcmBrushRev = 0xba,
    RainbowBrushClose = 0xbb,
    RainbowBrushOpen = 0xbc,
    RgbBrushClose = 0xbd,
    RgbBrushOpen = 0xbe,
    YcmBrushClose = 0xbf,
    YcmBrushOpen = 0xc0,
}

impl Effect {
    pub const ALL: [Self; 47] = [
        Self::AutoLoop,
        Self::RainbowFwd,
        Self::RainbowRev,
        Self::RainbowEnergy,
        Self::RainbowJump,
        Self::RgbJump,
        Self::YcmJump,
        Self::RainbowStrobe,
        Self::RgbStrobe,
        Self::YcmStrobe,
        Self::SevenFade,
        Self::RedYellowFade,
        Self::RedPurpleFade,
        Self::GreenCyanFade,
        Self::GreenYellowFade,
        Self::BluePurpleFade,
        Self::RedHorse,
        Self::GreenHorse,
        Self::BlueHorse,
        Self::YellowHorse,
        Self::CyanHorse,
        Self::PurpleHorse,
        Self::WhiteHorse,
        Self::RainbowFollowFwd,
        Self::RainbowFollowRev,
        Self::RgbFollowFwd,
        Self::RgbFollowRev,
        Self::YcmFollowFwd,
        Self::YcmFollowRev,
        Self::RainbowDriftFwd,
        Self::RainbowDriftRev,
        Self::RgbDriftFwd,
        Self::RgbDriftRev,
        Self::YcmDriftFwd,
        Self::YcmDriftRev,
        Self::RainbowBrushFwd,
        Self::RainbowBrushRev,
        Self::RgbBrushFwd,
        Self::RgbBrushRev,
        Self::YcmBrushFwd,
        Self::YcmBrushRev,
        Self::RainbowBrushClose,
        Self::RainbowBrushOpen,
        Self::RgbBrushClose,
        Self::RgbBrushOpen,
        Self::YcmBrushClose,
        Self::YcmBrushOpen,
    ];

    pub const fn byte(self) -> u8 {
        self as u8
    }

    pub const fn group(self) -> EffectGroup {
        match self {
            Self::AutoLoop
            | Self::RainbowFwd
            | Self::RainbowRev
            | Self::RainbowEnergy => EffectGroup::Rainbow,
            Self::RainbowJump
            | Self::RgbJump
            | Self::YcmJump
            | Self::RainbowStrobe
            | Self::RgbStrobe
            | Self::YcmStrobe
            | Self::SevenFade
            | Self::RedYellowFade
            | Self::RedPurpleFade
            | Self::GreenCyanFade
            | Self::GreenYellowFade
            | Self::BluePurpleFade => EffectGroup::Jump,
            Self::RedHorse
            | Self::GreenHorse
            | Self::BlueHorse
            | Self::YellowHorse
            | Self::CyanHorse
            | Self::PurpleHorse
            | Self::WhiteHorse => EffectGroup::Horse,
            Self::RainbowFollowFwd
            | Self::RainbowFollowRev
            | Self::RgbFollowFwd
            | Self::RgbFollowRev
            | Self::YcmFollowFwd
            | Self::YcmFollowRev => EffectGroup::Follow,
            Self::RainbowDriftFwd
            | Self::RainbowDriftRev
            | Self::RgbDriftFwd
            | Self::RgbDriftRev
            | Self::YcmDriftFwd
            | Self::YcmDriftRev => EffectGroup::Drift,
            Self::RainbowBrushFwd
            | Self::RainbowBrushRev
            | Self::RgbBrushFwd
            | Self::RgbBrushRev
            | Self::YcmBrushFwd
            | Self::YcmBrushRev
            | Self::RainbowBrushClose
            | Self::RainbowBrushOpen
            | Self::RgbBrushClose
            | Self::RgbBrushOpen
            | Self::YcmBrushClose
            | Self::YcmBrushOpen => EffectGroup::Brush,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::AutoLoop => "自动循环",
            Self::RainbowFwd => "正向幻彩",
            Self::RainbowRev => "反向幻彩",
            Self::RainbowEnergy => "七彩能量",
            Self::RainbowJump => "七彩跳变",
            Self::RgbJump => "红绿蓝跳变",
            Self::YcmJump => "黄青紫跳变",
            Self::RainbowStrobe => "七彩频闪",
            Self::RgbStrobe => "红绿蓝频闪",
            Self::YcmStrobe => "黄青紫频闪",
            Self::SevenFade => "七色渐变",
            Self::RedYellowFade => "红黄交替渐变",
            Self::RedPurpleFade => "红紫交替渐变",
            Self::GreenCyanFade => "绿青交替渐变",
            Self::GreenYellowFade => "绿黄交替渐变",
            Self::BluePurpleFade => "蓝紫交替渐变",
            Self::RedHorse => "红色跑马",
            Self::GreenHorse => "绿色跑马",
            Self::BlueHorse => "蓝色跑马",
            Self::YellowHorse => "黄色跑马",
            Self::CyanHorse => "青色跑马",
            Self::PurpleHorse => "紫色跑马",
            Self::WhiteHorse => "白色跑马",
            Self::RainbowFollowFwd => "正向七彩追光",
            Self::RainbowFollowRev => "反向七彩追光",
            Self::RgbFollowFwd => "正向红绿蓝追光",
            Self::RgbFollowRev => "反向红绿蓝追光",
            Self::YcmFollowFwd => "正向黄青紫追光",
            Self::YcmFollowRev => "反向黄青紫追光",
            Self::RainbowDriftFwd => "正向七彩飘动",
            Self::RainbowDriftRev => "反向七彩飘动",
            Self::RgbDriftFwd => "正向红绿蓝飘动",
            Self::RgbDriftRev => "反向红绿蓝飘动",
            Self::YcmDriftFwd => "正向黄青紫飘动",
            Self::YcmDriftRev => "反向黄青紫飘动",
            Self::RainbowBrushFwd => "正向七彩刷色",
            Self::RainbowBrushRev => "反向七彩刷色",
            Self::RgbBrushFwd => "正向红绿蓝刷色",
            Self::RgbBrushRev => "反向红绿蓝刷色",
            Self::YcmBrushFwd => "正向黄青紫刷色",
            Self::YcmBrushRev => "反向黄青紫刷色",
            Self::RainbowBrushClose => "七彩刷色闭幕",
            Self::RainbowBrushOpen => "七彩刷色拉幕",
            Self::RgbBrushClose => "红绿蓝刷色闭幕",
            Self::RgbBrushOpen => "红绿蓝刷色拉幕",
            Self::YcmBrushClose => "黄青紫刷色闭幕",
            Self::YcmBrushOpen => "黄青紫刷色拉幕",
        }
    }

    /// `7e 07 03 MODE 06 ff ff 00 ef`
    pub const fn frame(self) -> Frame {
        Frame::pack([0x07, 0x03, self.byte(), 0x06, 0xff, 0xff, 0x00])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_rainbow() {
        assert_eq!(Effect::RainbowFwd.frame().hex(), "7e 07 03 01 06 ff ff 00 ef");
        assert_eq!(Effect::RainbowRev.frame().hex(), "7e 07 03 02 06 ff ff 00 ef");
        assert_eq!(Effect::AutoLoop.frame().hex(), "7e 07 03 00 06 ff ff 00 ef");
    }

    #[test]
    fn all_unique() {
        let mut bytes: Vec<u8> = Effect::ALL.iter().map(|e| e.byte()).collect();
        bytes.sort();
        bytes.dedup();
        assert_eq!(bytes.len(), Effect::ALL.len());
    }
}
