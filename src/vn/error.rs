use std::error;
use std::fmt;

#[derive(Copy, Clone, Debug)]
pub enum Error {
    BadPayloadCount(u32),
    BadPayload,
    BadOpcode,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
        match self {
            Self::BadPayloadCount(u) => write!(f, "bad payload count: 0x{:08X}", u),
            Self::BadPayload => write!(f, "bad payload"),
            Self::BadOpcode => write!(f, "bad opcode"),
        }
    }
}

impl error::Error for Error {}
