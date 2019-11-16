extern crate rusb;

#[derive(Debug)]
pub enum PolarError {
    LibusbError { error: rusb::Error },
    PolarError { error: String },
}

impl PolarError {
    pub fn new<S>(message: S) -> PolarError
    where
        S: Into<String>,
    {
        PolarError::PolarError {
            error: message.into(),
        }
    }
}

impl From<rusb::Error> for PolarError {
    fn from(error: rusb::Error) -> PolarError {
        PolarError::LibusbError { error }
    }
}
