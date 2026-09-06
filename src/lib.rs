//! hi-my-light BLE 后端。

pub mod device;
pub mod error;
pub mod manager;
pub mod protocol;
pub mod shutdown_off;
pub mod shutdown_once;

pub use device::{CharacteristicInfo, DeviceInfo};
pub use error::BleError;
pub use manager::{BleEvent, BleManager};
pub use protocol::{
    Brightness, Cct, Channel, Clock, Command, Effect, EffectGroup, Frame, Level, Power, Query, Rgb,
    Speed, Switch, WhiteMix,
};
pub use shutdown_off::shutdown_turn_off;
