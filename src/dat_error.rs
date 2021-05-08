use std::array;
use std::convert::From;
use std::error::Error;
use std::fmt;
use std::io;
use std::num;

/// Wrapper error for any error related to processing a binary DAT file.
#[derive(Debug)]
pub enum DATError {
    /// The header data is incorrect. The file is probably not a binary DAT file,
    /// but may be a plaintext DAT.
    BadHeader(&'static str),
    /// Content length exceeds the maximum possible length specified in the header, or the 
    /// maximum possible length.
    ContentOverflow(&'static str),
    /// Wrapper for various `std::io::Error` errors. Represents an error reading or writing the
    /// file on disk.
    FileIO(io::Error),
    /// Attempted to use a type-specific function on the incorrect [`DATType`](crate::dat_type::DATType)
    IncorrectType(&'static str),
}

impl fmt::Display for DATError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DATError::BadHeader(desc) => write!(f, "Invalid header data: {}", desc),
            DATError::ContentOverflow(desc) => write!(f, "Content overflow: {}", desc),
            DATError::FileIO(e) => write!(f, "File IO error: {:?}", e.source()),
            DATError::IncorrectType(desc) => write!(f, "Incorrect DAT file type: {}", desc),
        }
    }
}

impl Error for DATError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            DATError::FileIO(ref e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for DATError {
    fn from(e: io::Error) -> DATError {
        DATError::FileIO(e)
    }
}

impl From<num::TryFromIntError> for DATError {
    fn from(_: num::TryFromIntError) -> DATError {
        DATError::ContentOverflow("Could not index full file content on a 16-bit platform.")
    }
}

impl From<array::TryFromSliceError> for DATError {
    fn from(_: array::TryFromSliceError) -> DATError {
        DATError::BadHeader("Header data is absent or unreadable.")
    }
}
