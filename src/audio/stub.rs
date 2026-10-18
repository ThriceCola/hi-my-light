use super::{Error, Sample};

pub const SUPPORTED: bool = false;

pub struct Capture;

impl Capture {
    pub fn start() -> Result<Self, Error> {
        Err(Error::Unsupported)
    }

    pub fn try_sample(&self) -> Result<Option<Sample>, Error> {
        Ok(None)
    }

    pub fn set_beat(&self, _on: bool) {}

    pub fn beat_ready(&self) -> bool {
        false
    }

    pub fn beat_error(&self) -> Option<String> {
        Some("这一系统还没有节拍拾音".into())
    }

    pub fn clear_beat(&self, _seq: u64) {}

    pub fn reset_beat(&self) {}
}
