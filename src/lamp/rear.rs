use hi_my_light::{Channel, Command, Effect, Frame, Level, Rgb, Speed};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RearLook {
    Solid(Rgb),
    Play(Effect),
    Audio,
}

#[derive(Clone, Copy, Debug)]
pub struct Rear {
    pub on: bool,
    pub level: Level,
    pub look: RearLook,
    pub speed: Level,
    pub solid: Rgb,
    pub effect: Effect,
    lit: bool,
}

impl Rear {
    pub fn new(on: bool, level: Level, look: RearLook, speed: Level) -> Self {
        let (solid, effect) = match look {
            RearLook::Solid(color) => (color, Effect::RainbowFwd),
            RearLook::Play(effect) => (Rgb::new(0xE0, 0xA8, 0x5C), effect),
            RearLook::Audio => (Rgb::new(0xE0, 0xA8, 0x5C), Effect::RainbowFwd),
        };
        Self {
            on,
            level,
            look,
            speed,
            solid,
            effect,
            lit: false,
        }
    }

    pub fn with_memory(mut self, solid: Rgb, effect: Effect) -> Self {
        self.solid = solid;
        self.effect = effect;
        self
    }

    pub fn extinguish(&mut self) {
        self.lit = false;
    }

    pub fn sync_lit(&mut self) {
        self.lit = self.on;
    }

    pub fn set_level(&mut self, level: Level) {
        self.level = level;
        self.on = true;
    }

    pub fn set_rgb(&mut self, rgb: Rgb) {
        self.solid = rgb;
        self.look = RearLook::Solid(rgb);
        self.on = true;
    }

    pub fn set_effect(&mut self, effect: Effect) {
        self.effect = effect;
        self.look = RearLook::Play(effect);
        self.on = true;
    }

    pub fn show_solid(&mut self) {
        self.look = RearLook::Solid(self.solid);
    }

    pub fn show_effect(&mut self) {
        self.look = RearLook::Play(self.effect);
    }

    pub fn show_audio(&mut self) {
        self.look = RearLook::Audio;
        self.on = true;
    }

    pub fn toggle(&mut self) {
        self.on = !self.on;
        if self.on && self.level < Level::from_byte(8) {
            self.level = Level::from_byte(40);
        }
    }

    pub fn arm(&mut self) -> Option<Frame> {
        if !self.on {
            self.lit = false;
            return None;
        }
        if self.lit {
            return None;
        }
        self.lit = true;
        Some(self.power_frame())
    }

    pub fn power_frame(&self) -> Frame {
        Command::power(Channel::Rear, self.on).frame()
    }

    pub fn brightness_frame(&self) -> Frame {
        Command::brightness(Channel::Rear, self.level).frame()
    }

    pub fn look_frame(&self) -> Frame {
        match self.look {
            RearLook::Solid(rgb) => rgb.frame(),
            RearLook::Play(effect) => effect.frame(),
            RearLook::Audio => Rgb::BLACK.scroll_frame(),
        }
    }

    pub fn speed_frame(&self) -> Frame {
        Speed::new(self.speed).frame()
    }

    pub fn wake_frames(&mut self) -> Vec<Frame> {
        if !self.on {
            self.lit = false;
            return vec![self.power_frame()];
        }
        let mut frames = Vec::new();
        if !self.lit {
            frames.push(self.power_frame());
            self.lit = true;
        }
        frames.push(self.look_frame());
        frames.push(self.brightness_frame());
        if matches!(self.look, RearLook::Play(_)) {
            frames.push(self.speed_frame());
        }
        frames
    }
}
