use super::{
    Brightness, Cct, Channel, Clock, Effect, Frame, Level, Power, Query, Rgb, Speed, Switch,
    WhiteMix,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Power(Power),
    Brightness(Brightness),
    Cct(Cct),
    WhiteMix(WhiteMix),
    Rgb(Rgb),
    Effect(Effect),
    Speed(Speed),
    Query(Query),
    Clock(Clock),
}

impl Command {
    pub const fn front_on() -> Self {
        Self::Power(Power::on(Channel::Front))
    }

    pub const fn front_off() -> Self {
        Self::Power(Power::off(Channel::Front))
    }

    pub const fn rear_on() -> Self {
        Self::Power(Power::on(Channel::Rear))
    }

    pub const fn rear_off() -> Self {
        Self::Power(Power::off(Channel::Rear))
    }

    pub const fn power(channel: Channel, on: bool) -> Self {
        let switch = if on { Switch::On } else { Switch::Off };
        Self::Power(Power { channel, switch })
    }

    pub const fn brightness(channel: Channel, level: Level) -> Self {
        Self::Brightness(Brightness::new(channel, level))
    }

    pub fn frame(self) -> Frame {
        match self {
            Self::Power(cmd) => cmd.frame(),
            Self::Brightness(cmd) => cmd.frame(),
            Self::Cct(cmd) => cmd.frame(),
            Self::WhiteMix(cmd) => cmd.frame(),
            Self::Rgb(cmd) => cmd.frame(),
            Self::Effect(cmd) => cmd.frame(),
            Self::Speed(cmd) => cmd.frame(),
            Self::Query(cmd) => cmd.frame(),
            Self::Clock(cmd) => cmd.frame(),
        }
    }
}
