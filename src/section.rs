use std::convert::{TryFrom, TryInto};
use std::io::Read;

use crate::dat_error::DATError;
use crate::dat_file::DATFile;
use crate::dat_type::DATType;
use std::cmp::Ordering;
use std::str::from_utf8;

/// Array of [`DATTypes`](crate::dat_type::DATType) that have `Section`-based contents.
pub const SECTION_BASED_TYPES: [DATType; 4] = [DATType::ACQ, DATType::KEYBIND, DATType::MACRO, DATType::MACROSYS];

/// Length of a section header in bytes.
pub const SECTION_HEADER_SIZE: usize = 3;

/// A `Section` is variable-length data structure common to several binary DAT files.
/// A `Resource` (ie, a Macro or Gearset) is then made out of a repeating pattern of `Section`s.
///
/// [`Section`] owns its constituent data and is returned from helper functions like [`read_section()`].
/// To build a section with refrences to a pre-allocated buffer, use [`SectionData`].
///
/// # Section-using file types
/// `ACQ`, `KEYBIND`, `MACRO`, and `MACROSYS`. See [`SECTION_BASED_TYPES`].
///
/// # Data Structure
/// ```text
/// 0
/// 0  1  2  3  ...
/// |  |--|  |- ...
/// |  |     \_ null-terminated utf8 string
/// |  \_ u16le content_size
/// \_ utf8 char section_type
/// ```
#[derive(Clone, Debug)]
pub struct Section {
    /// Data content of the section.
    pub content: String,
    /// Length of section content in bytes. Includes terminating null.
    pub content_size: u16,
    /// Single char string data type tag. The meaning of this tag varies by file type.
    /// Some tags are reused with different meanings between types.
    pub tag: String,
}

/// A `Section` is variable-length data structure common to several binary DAT files.
/// A `Resource` (ie, a Macro or Gearset) is then made out of a repeating pattern of `Section`s.
///
/// [`SectionData`] is used to build Sections with references to pre-allocated buffers.
/// To build a section that owns its own data, use [`Section`].
///
/// # Section-using file types
/// `ACQ`, `KEYBIND`, `MACRO`, and `MACROSYS`. See [`SECTION_BASED_TYPES`].
///
/// # Data Structure
/// ```text
/// 0
/// 0  1  2  3  ...
/// |  |--|  |- ...
/// |  |     \_ null-terminated utf8 string
/// |  \_ u16le content_size
/// \_ utf8 char section_type
/// ```
#[derive(Clone, Debug)]
pub struct SectionData<'a> {
    /// Data content of the section.
    pub content: &'a str,
    /// Length of section content in bytes. Includes terminating null.
    pub content_size: u16,
    /// Single char string data type tag. The meaning of this tag varies by file type.
    /// Some tags are reused with different meanings between types.
    pub tag: &'a str,
}

impl From<Section> for Vec<u8> {
    fn from(x: Section) -> Self {
        let tag_bytes = x.tag.as_bytes();
        let content_size_bytes = x.content_size.to_le_bytes();
        let content_bytes = x.content.as_bytes();
        tag_bytes
            .iter()
            .chain(&content_size_bytes)
            .chain(content_bytes)
            .chain(&[0u8; 1])
            .copied()
            .collect()
    }
}

impl TryFrom<&[u8]> for Section {
    type Error = DATError;
    fn try_from(x: &[u8]) -> Result<Self, Self::Error> {
        let header_bytes = x[0..SECTION_HEADER_SIZE].try_into()?;
        let (tag, content_size) = get_section_header_contents(&header_bytes)?;
        let remaining_buf_size = x.len() - 3;

        match usize::from(content_size).cmp(&remaining_buf_size) {
            Ordering::Greater => Err(DATError::ContentUnderflow(
                "Data buffer is too small for content_size specified in header.",
            )),
            Ordering::Less => Err(DATError::ContentOverflow(
                "Data buffer is too large for content_size specified in header.",
            )),
            Ordering::Equal => Ok(Section {
                content: String::from_utf8(x[3..].to_vec())?,
                content_size,
                tag: tag.to_owned(),
            }),
        }
    }
}

impl From<SectionData<'_>> for Vec<u8> {
    fn from(x: SectionData) -> Self {
        let tag_bytes = x.tag.as_bytes();
        let content_size_bytes = x.content_size.to_le_bytes();
        let content_bytes = x.content.as_bytes();
        tag_bytes
            .iter()
            .chain(&content_size_bytes)
            .chain(content_bytes)
            .chain(&[0u8; 1])
            .copied()
            .collect()
    }
}

impl<'a> TryFrom<&'a [u8]> for SectionData<'a> {
    type Error = DATError;
    fn try_from(x: &'a [u8]) -> Result<Self, Self::Error> {
        let tag = from_utf8(&x[..1])?;
        let content_size = u16::from_le_bytes(x[1..SECTION_HEADER_SIZE].try_into()?);
        let remaining_buf_size = x.len() - 3;

        match usize::from(content_size).cmp(&remaining_buf_size) {
            Ordering::Greater => Err(DATError::ContentUnderflow(
                "Data buffer is too small for content_size specified in header.",
            )),
            Ordering::Less => Err(DATError::ContentOverflow(
                "Data buffer is too large for content_size specified in header.",
            )),
            Ordering::Equal => Ok(SectionData {
                content: from_utf8(&x[3..])?,
                content_size,
                tag,
            }),
        }
    }
}

/// Tries to read a [`SECTION_HEADER_SIZE`] byte array as a [`Section`] header.
/// Returns a tuple containing (`tag`, `content_size`).
///
/// # Errors
/// This function will return a [`DATError::BadEncoding`](crate::dat_error::DATError::BadEncoding) if the
/// `tag` is not a valid utf8 character.
///
/// # Examples
/// ```rust
/// use libxivdat::section::get_section_header_contents;
///
/// let bytes = [97, 01, 00];
/// let (tag, content_size) = get_section_header_contents(&bytes).unwrap();
/// assert_eq!(tag, "a");
/// assert_eq!(content_size, 1);
/// ```
pub fn get_section_header_contents(bytes: &[u8; SECTION_HEADER_SIZE]) -> Result<(&str, u16), DATError> {
    Ok((from_utf8(&bytes[..1])?, u16::from_le_bytes(bytes[1..].try_into()?)))
}

//
// pub fn read_section(dat_file: &mut DATFile) -> Result<Section, DATError> {

// }
