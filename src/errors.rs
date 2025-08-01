use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct Error {
    pub message: String,
}

impl Error for Error {
    fn description(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}