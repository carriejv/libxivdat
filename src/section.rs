use std::convert::{TryFrom, TryInto};
use std::io::{Read, Seek, SeekFrom};

use crate::dat_error::DATError;
use crate::dat_file::{read_content, DATFile};
use crate::dat_type::DATType;
use std::cmp::Ordering;
use std::path::Path;
use std::str::from_utf8;

/// Array of [`DATTypes`](crate::dat_type::DATType) that have `Section`-based contents.
pub const SECTION_BASED_TYPES: [DATType; 4] = [DATType::ACQ, DATType::KEYBIND, DATType::MACRO, DATType::MACROSYS];

/// Length of a section header in bytes.
pub const SECTION_HEADER_SIZE: usize = 3;

/// A `Section` is variable-length data structure common to several binary DAT files.
/// A `Resource` (ie, a Macro or Gearset) is then made out of a repeating pattern of sections.
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
/// A `Resource` (ie, a Macro or Gearset) is then made out of a repeating pattern of sections.
/// [`SectionData`] is used to build sections with references to pre-allocated buffers.
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

impl From<&SectionData<'_>> for Section {
    fn from(x: &SectionData) -> Self {
        Section {
            content: x.content.to_owned(),
            content_size: x.content_size,
            tag: x.tag.to_owned(),
        }
    }
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
                content: String::from_utf8(x[3..x.len() - 1].to_vec())?,
                content_size,
                tag: tag.to_owned(),
            }),
        }
    }
}

impl<'a> From<&'a Section> for SectionData<'a> {
    fn from(x: &'a Section) -> Self {
        SectionData {
            content: &x.content,
            content_size: x.content_size,
            tag: &x.tag,
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
                content: from_utf8(&x[3..x.len() - 1])?,
                content_size,
                tag,
            }),
        }
    }
}

/// Interprets a byte slice as [`SectionData`].
///
/// # Errors
///
/// Returns a [`DATError::ContentOverflow`](crate::dat_error::DATError::ContentOverflow) or
/// [`DATError::ContentUnderflow`](crate::dat_error::DATError::ContentUnderflow) if the slice length
/// does not match the content_size specified in the section header.
///
/// If the tag or content is not valid utf8 text, a [`DATError::BadEncoding`](crate::dat_error::DATError::BadEncoding)
/// error will be returned.
///
/// # Examples
/// ```rust
/// use libxivdat::dat_file::DATFile;
/// use libxivdat::section::{as_section,SECTION_HEADER_SIZE};
/// use std::io::Read;
///
/// let mut dat_file = DATFile::open("./resources/TEST_SECTION.DAT").unwrap();
/// let mut content_bytes = [0u8; 24 + SECTION_HEADER_SIZE];
/// dat_file.read_exact(&mut content_bytes).unwrap();
/// let section = as_section(&content_bytes).unwrap();
///
/// assert_eq!(section.tag, "T");
/// assert_eq!(section.content_size, 24);
/// assert_eq!(section.content, "This is a test section.");
/// ```
pub fn as_section(bytes: &[u8]) -> Result<SectionData, DATError> {
    SectionData::try_from(bytes)
}

/// Interprets a byte slice as a block of [`SectionData`], returning a [`Vec`] of them.
///
/// # Errors
///
/// Returns a [`DATError::ContentOverflow`](crate::dat_error::DATError::ContentOverflow) or
/// [`DATError::ContentUnderflow`](crate::dat_error::DATError::ContentUnderflow) if a section content block
/// does not match the expected length specified in the section header.
///
/// If the tag or content is not valid utf8 text, a [`DATError::BadEncoding`](crate::dat_error::DATError::BadEncoding)
/// error will be returned.
///
/// # Examples
/// ```rust
/// use libxivdat::dat_file::read_content;
/// use libxivdat::section::as_section_vec;
///
/// let content_bytes = read_content("./resources/TEST_SECTION.DAT").unwrap();
/// let section = as_section_vec(&content_bytes).unwrap();
///
/// assert_eq!(section[0].tag, "T");
/// assert_eq!(section[0].content_size, 24);
/// assert_eq!(section[0].content, "This is a test section.");
///
/// assert_eq!(section[1].tag, "A");
/// assert_eq!(section[1].content_size, 22);
/// assert_eq!(section[1].content, "Another test section.");
/// ```
pub fn as_section_vec<'a>(bytes: &'a [u8]) -> Result<Vec<SectionData<'a>>, DATError> {
    let mut cursor = 0usize;
    let mut res_vec = Vec::<SectionData<'a>>::new();
    let buf_len = bytes.len();
    while cursor < buf_len {
        // Read header block
        let header_bytes = &bytes[cursor..cursor + SECTION_HEADER_SIZE];
        let (tag, content_size) = get_section_header_contents(header_bytes.try_into()?)?;
        cursor += SECTION_HEADER_SIZE;
        // Read content block; leave the terminating null out of the content slice
        let content_bytes = &bytes[cursor..cursor + usize::from(content_size) - 1];
        cursor += usize::from(content_size);
        // Validate content size
        if content_bytes.contains(&0u8) {
            return Err(DATError::ContentUnderflow("Section content ended early."));
        }
        if bytes[cursor - 1] != 0u8 {
            return Err(DATError::ContentOverflow(
                "Section data did not end at the expected index.",
            ));
        }
        // Build a section and push to vec
        res_vec.push(SectionData::<'a> {
            content: from_utf8(content_bytes)?,
            content_size,
            tag,
        });
    }
    Ok(res_vec)
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

/// Reads the next [`Section`] from a [`DATFile`](crate::dat_file::DATFile).
///
/// # Errors
///
/// If an I/O error occurs while writing to the file, a [`DATError::FileIO`](crate::dat_error::DATError::FileIO)
/// error will be returned wrapping the underlying FS error.
///
/// If the tag or content is not valid utf8 text, a [`DATError::BadEncoding`](crate::dat_error::DATError::BadEncoding)
/// error will be returned.
///
/// # Examples
/// ```rust
/// use libxivdat::dat_file::DATFile;
/// use libxivdat::section::read_section;
///
/// let mut dat_file = DATFile::open("./resources/TEST_SECTION.DAT").unwrap();
/// let section = read_section(&mut dat_file).unwrap();
///
/// assert_eq!(section.tag, "T");
/// assert_eq!(section.content_size, 24);
/// assert_eq!(section.content, "This is a test section.");
///
/// ```
pub fn read_section(dat_file: &mut DATFile) -> Result<Section, DATError> {
    // Read section header.
    let mut sec_header_bytes = [0u8; SECTION_HEADER_SIZE];
    dat_file.read_exact(&mut sec_header_bytes)?;
    let (tag, content_size) = get_section_header_contents(&sec_header_bytes)?;
    // Read section content
    let mut sec_content_bytes = vec![0u8; usize::from(content_size - 1)];
    dat_file.read_exact(&mut sec_content_bytes)?;
    // Skip null byte. Doing it this way avoids having to re-slice content bytes.
    dat_file.seek(SeekFrom::Current(1))?;
    Ok(Section {
        content: String::from_utf8(sec_content_bytes)?,
        content_size,
        tag: tag.to_owned(),
    })
}

/// Reads all [`Sections`](Section) from a specified DAT file, returning a [`Vec`] of them.
/// This performs only one read operation on the underlying file, loading the entire content into memory
/// to prevent repeat file access. This is similar to [`read_content()`](crate::dat_file::read_content),
/// but returns a `Vec<Section>` instead of raw bytes.
///
/// # Errors
///
/// Returns a [`DATError::ContentOverflow`](crate::dat_error::DATError::ContentOverflow) or
/// [`DATError::ContentUnderflow`](crate::dat_error::DATError::ContentUnderflow) if a section content block
/// does not match the expected length specified in the section header.
///
/// Returns a [`DATError::BadEncoding`](crate::dat_error::DATError::BadEncoding) if a section does not
/// contain valid utf8 text.
///
/// Returns a [`DATError::BadHeader`](crate::dat_error::DATError::BadHeader) if the specified file does not
/// have a valid DAT header.
///
/// If an I/O error occurs while writing to the file, a [`DATError::FileIO`](crate::dat_error::DATError::FileIO)
/// error will be returned wrapping the underlying FS error.
///
/// # Examples
/// ```rust
/// use libxivdat::section::read_section_content;
///
/// let section = read_section_content("./resources/TEST_SECTION.DAT").unwrap();
///
/// assert_eq!(section[0].tag, "T");
/// assert_eq!(section[0].content_size, 24);
/// assert_eq!(section[0].content, "This is a test section.");
///
/// assert_eq!(section[1].tag, "A");
/// assert_eq!(section[1].content_size, 22);
/// assert_eq!(section[1].content, "Another test section.");
/// ```
pub fn read_section_content<P: AsRef<Path>>(path: P) -> Result<Vec<Section>, DATError> {
    let content_bytes = read_content(path)?;
    let section_data = as_section_vec(&content_bytes)?;
    Ok(section_data.iter().map(Section::from).collect())
}
