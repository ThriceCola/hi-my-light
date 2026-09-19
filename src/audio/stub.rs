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
}
