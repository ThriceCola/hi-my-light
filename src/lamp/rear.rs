use hi_my_light::{Channel, Command, Effect, Frame, Level, Rgb, Speed};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RearLook {
    Solid(Rgb),
    Play(Effect),
}

#[derive(Clone, Copy, Debug)]
pub struct Rear {
    pub on: bool,
    pub level: Level,
    pub look: RearLook,
    pub speed: Level,
    lit: bool,
}

impl Rear {
    pub fn new(on: bool, level: Level, look: RearLook, speed: Level) -> Self {
        Self {
            on,
            level,
            look,
            speed,
            lit: false,
        }
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
        self.look = RearLook::Solid(rgb);
        self.on = true;
    }

    pub fn set_effect(&mut self, effect: Effect) {
        self.look = RearLook::Play(effect);
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

    pub fn rgb(&self) -> Rgb {
        match self.look {
            RearLook::Solid(rgb) => rgb,
            RearLook::Play(_) => Rgb::WHITE,
        }
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
