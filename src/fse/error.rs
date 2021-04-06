use std::error;
use std::fmt;

#[derive(Copy, Clone, Debug)]
pub enum Error {
    BadBitStream,
    BadLiteralBits,
    BadLiteralCount(u32),
    BadLiteralPayload,
    BadLiteralState,
    BadLmdBits,
    BadLmdCount(u32),
    BadLmdPayload,
    BadLmdState,
    BadPayloadCount,
    BadRawByteCount,
    BadReaderState,
    BadWeightPayload,
    BadWeightPayloadCount,
    WeightPayloadOverflow,
    WeightPayloadUnderflow,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
        match self {
            Self::BadBitStream => write!(f, "bad bitstream"),
            Self::BadLiteralBits => write!(f, "bad literal bits"),
            Self::BadLiteralCount(u) => write!(f, "bad literal count: 0x{:08X}", u),
            Self::BadLiteralPayload => write!(f, "bad literal payload"),
            Self::BadLiteralState => write!(f, "bad literal state"),
            Self::BadLmdBits => write!(f, "bad lmd bits"),
            Self::BadLmdCount(u) => write!(f, "bad lmd count: 0x{:08X}", u),
            Self::BadLmdPayload => write!(f, "bad lmd payload"),
            Self::BadLmdState => write!(f, "bad lmd state"),
            Self::BadPayloadCount => write!(f, "bad payload count"),
            Self::BadRawByteCount => write!(f, "bad raw byte count"),
            Self::BadReaderState => write!(f, "bad reader state"),
            Self::BadWeightPayload => write!(f, "bad weight payload"),
            Self::BadWeightPayloadCount => write!(f, "bad weight payload count"),
            Self::WeightPayloadOverflow => write!(f, "weight payload overflow"),
            Self::WeightPayloadUnderflow => write!(f, "weight payload underflow"),
        }
    }
}

impl error::Error for Error {}
