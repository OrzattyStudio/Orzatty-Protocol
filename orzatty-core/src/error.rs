use core::fmt;

/// Errors that can occur when processing Orzatty frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The destination buffer is too small to hold the encoded data.
    BufferTooSmall { needed: usize, available: usize },
    /// The input buffer is too short to contain a valid header or varint.
    IncompleteInput { needed_min: usize, available: usize },
    /// The frame type byte is invalid or unknown.
    InvalidFrameType(u8),
    /// The VarInt encoding is invalid (e.g., overflows 64 bits or is malformed).
    InvalidVarInt,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BufferTooSmall { needed, available } => 
                write!(f, "Buffer too small: needed {} bytes, but only {} available", needed, available),
            Error::IncompleteInput { needed_min, available } => 
                write!(f, "Incomplete input: need at least {} bytes, but only {} available", needed_min, available),
            Error::InvalidFrameType(t) => 
                write!(f, "Invalid frame type: {:#04x}", t),
            Error::InvalidVarInt => 
                write!(f, "Invalid VarInt encoding"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
