use std::array;
use std::convert::From;
use std::error::Error;
use std::fmt;
use std::io;
use std::num;

/// Wrapper error for any error related to processing a binary DAT file.
#[derive(Debug)]
pub enum DATError {
    /// Attempted to read a byte stream as UTF-8 text when it didn't contain
    /// valid UTF-8.
    BadEncoding(&'static str),
    /// The header data is incorrect. The file is probably not a binary DAT file,
    /// but may be a plaintext DAT.
    BadHeader(&'static str),
    /// Data provided exceeds the maximum length specified in the header or the
    /// maximum possible length.
    Overflow(&'static str),
    /// Data provided is shorter than the content_size specified in the header or
    /// the minimum possible length.
    Underflow(&'static str),
    /// Unexpectedly hit the EOF when attempting to read a block of data.
    EndOfFile(&'static str),
    /// Wrapper for various `std::io::Error` errors. Represents an error reading or writing a
    /// file on disk.
    FileIO(io::Error),
    /// Attempted to use a type-specific function on the incorrect [`DATType`](crate::dat_type::DATType)
    IncorrectType(&'static str),
    /// Invalid input for a function
    InvalidInput(&'static str),
}

impl fmt::Display for DATError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DATError::BadEncoding(desc) => write!(f, "Invalid text encoding: {}", desc),
            DATError::BadHeader(desc) => write!(f, "Invalid header data: {}", desc),
            DATError::Overflow(desc) => write!(f, "Content overflow: {}", desc),
            DATError::Underflow(desc) => write!(f, "Content underflow: {}", desc),
            DATError::EndOfFile(desc) => write!(f, "Unexpected EOF: {}", desc),
            DATError::FileIO(e) => write!(f, "File IO error: {:?}", e.source()),
            DATError::IncorrectType(desc) => write!(f, "Incorrect DAT file type: {}", desc),
            DATError::InvalidInput(desc) => write!(f, "Invalid input: {}", desc),
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
        DATError::Overflow("Could not index full file content on a 16-bit platform.")
    }
}

impl From<array::TryFromSliceError> for DATError {
    fn from(_: array::TryFromSliceError) -> DATError {
        DATError::BadHeader("Header data is absent or unreadable.")
    }
}

impl From<std::str::Utf8Error> for DATError {
    fn from(_: std::str::Utf8Error) -> DATError {
        DATError::BadEncoding("Text data block did not contain valid utf-8.")
    }
}

impl From<std::string::FromUtf8Error> for DATError {
    fn from(_: std::string::FromUtf8Error) -> DATError {
        DATError::BadEncoding("Text data block did not contain valid utf-8.")
    }
}
