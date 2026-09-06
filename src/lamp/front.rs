use hi_my_light::{Cct, Channel, Command, Frame, Level};

#[derive(Clone, Copy, Debug)]
pub struct Front {
    pub on: bool,
    pub level: Level,
    pub cct: Cct,
    lit: bool,
}

impl Front {
    pub fn new(on: bool, level: Level, cct: Cct) -> Self {
        Self {
            on,
            level,
            cct,
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
        Command::power(Channel::Front, self.on).frame()
    }

    pub fn brightness_frame(&self) -> Frame {
        Command::brightness(Channel::Front, self.level).frame()
    }

    pub fn cct_frame(&self) -> Frame {
        self.cct.frame()
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
        frames.push(self.brightness_frame());
        frames.push(self.cct_frame());
        frames
    }
}
