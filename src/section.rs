use std::convert::{TryFrom, TryInto};
use std::io::{Read, Seek, SeekFrom};

use crate::dat_error::DATError;
use crate::dat_file::{check_type, read_content, DATFile};
use crate::dat_type::DATType;
use std::cmp::Ordering;
use std::path::Path;
use std::str::from_utf8;

/// Array of [`DATTypes`](crate::dat_type::DATType) that have `Section`-based contents. [`DATType::Unknown`] is allowed,
/// since its contents are not known.
pub const SECTION_BASED_TYPES: [DATType; 5] = [
    DATType::ACQ,
    DATType::KEYBIND,
    DATType::MACRO,
    DATType::MACROSYS,
    DATType::Unknown,
];

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
#[derive(Clone, Debug, Eq, PartialEq)]
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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
            Ordering::Greater => Err(DATError::Overflow(
                "Data buffer is too small for content_size specified in header.",
            )),
            Ordering::Less => Err(DATError::Underflow(
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
            Ordering::Greater => Err(DATError::Overflow(
                "Data buffer is too small for content_size specified in header.",
            )),
            Ordering::Less => Err(DATError::Underflow(
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

impl Section {
    /// Builds a new [`Section`] with a given tag and content
    ///
    /// # Examples
    /// ```rust
    /// use libxivdat::section::Section;
    ///
    /// let new_section = Section::new("T".to_string(), "Macro title!".to_string()).unwrap();
    /// assert_eq!(new_section.tag, "T");
    /// assert_eq!(new_section.content, "Macro title!");
    /// assert_eq!(new_section.content_size, 13);
    /// ```
    pub fn new(tag: String, content: String) -> Result<Self, DATError> {
        if tag.len() != 1 {
            return Err(DATError::InvalidInput("Tags may only be a single character in length."));
        }
        // Include space for terminating null
        let content_size = match u16::try_from(content.len() + 1) {
            Ok(content_size) => content_size,
            Err(_) => {
                return Err(DATError::Overflow(
                    "Section content exceeds maximum possible size (u16::MAX - 1).",
                ))
            }
        };
        Ok(Section {
            content,
            content_size,
            tag,
        })
    }
}

impl<'a> SectionData<'a> {
    /// Builds a new [`SectionData`] with a given tag and content
    ///
    /// # Examples
    /// ```rust
    /// use libxivdat::section::SectionData;
    ///
    /// let new_section = SectionData::new("T", "Macro title!").unwrap();
    /// assert_eq!(new_section.tag, "T");
    /// assert_eq!(new_section.content, "Macro title!");
    /// assert_eq!(new_section.content_size, 13);
    /// ```
    pub fn new(tag: &'a str, content: &'a str) -> Result<Self, DATError> {
        if tag.len() != 1 {
            return Err(DATError::InvalidInput("Tags may only be a single character in length."));
        }
        // Include space for terminating null
        let content_size = match u16::try_from(content.len() + 1) {
            Ok(content_size) => content_size,
            Err(_) => {
                return Err(DATError::Overflow(
                    "Section content exceeds maximum possible size (u16::MAX - 1).",
                ))
            }
        };
        Ok(SectionData::<'a> {
            content,
            content_size,
            tag,
        })
    }
}

/// Interprets a byte slice as [`SectionData`].
///
/// # Errors
///
/// Returns a [`DATError::Overflow`](crate::dat_error::DATError::Overflow) or
/// [`DATError::Underflow`](crate::dat_error::DATError::Underflow) if the slice length
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
/// Returns a [`DATError::Overflow`](crate::dat_error::DATError::Overflow) or
/// [`DATError::Underflow`](crate::dat_error::DATError::Underflow) if a section content block
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
            return Err(DATError::Underflow("Section content ended early."));
        }
        if bytes[cursor - 1] != 0u8 {
            return Err(DATError::Overflow("Section data did not end at the expected index."));
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
/// Returns [`DATError::IncorrectType`] if the file appears to be of a [`DATType`]
/// that does not contain sections.
///
/// Returns [`DATError::EndOfFile`] if there is not a full section remaining in the file.
///
/// If an I/O error occurs while reading the file, a [`DATError::FileIO`]
/// error will be returned wrapping the underlying FS error.
///
/// If the tag or content is not valid utf8 text, a [`DATError::BadEncoding`]
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
    if SECTION_BASED_TYPES.contains(&dat_file.file_type()) {
        Ok(read_section_unsafe(dat_file)?)
    } else {
        Err(DATError::IncorrectType(
            "Target file is of a type that should not contain sections.",
        ))
    }
}

/// Reads all [`Sections`](Section) from a specified DAT file, returning a [`Vec`] of them.
/// This performs only one read operation on the underlying file, loading the entire content into memory
/// to prevent repeat file access. This is similar to [`read_content()`](crate::dat_file::read_content),
/// but returns a `Vec<Section>` instead of raw bytes.
///
/// # Errors
///
/// Returns [`DATError::IncorrectType`] if the file appears to be of a [`DATType`]
/// that does not contain sections.
///
/// Returns a [`DATError::Overflow`](crate::dat_error::DATError::Overflow) or
/// [`DATError::Underflow`](crate::dat_error::DATError::Underflow) if a section content block
/// does not match the expected length specified in the section header.
///
/// Returns a [`DATError::BadEncoding`](crate::dat_error::DATError::BadEncoding) if a section does not
/// contain valid utf8 text.
///
/// Returns a [`DATError::BadHeader`](crate::dat_error::DATError::BadHeader) if the specified file does not
/// have a valid DAT header.
///
/// If an I/O error occurs while reading the file, a [`DATError::FileIO`](crate::dat_error::DATError::FileIO)
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
    if SECTION_BASED_TYPES.contains(&check_type(&path)?) {
        Ok(read_section_content_unsafe(path)?)
    } else {
        Err(DATError::IncorrectType(
            "Target file is of a type that should not contain sections.",
        ))
    }
}

/// Reads the next [`Section`] from a [`DATFile`](crate::dat_file::DATFile). This does not check that the
/// file is of a type that should contain sections.
///
/// # Errors
///
/// Returns [`DATError::EndOfFile`] if there is not a full section remaining in the file.
///
/// If an I/O error occurs while reading the file, a [`DATError::FileIO`]
/// error will be returned wrapping the underlying FS error.
///
/// If the tag or content is not valid utf8 text, a [`DATError::BadEncoding`]
/// error will be returned.
///
/// # Examples
/// ```rust
/// use libxivdat::dat_file::DATFile;
/// use libxivdat::section::read_section_unsafe;
///
/// let mut dat_file = DATFile::open("./resources/TEST_SECTION.DAT").unwrap();
/// let section = read_section_unsafe(&mut dat_file).unwrap();
///
/// assert_eq!(section.tag, "T");
/// assert_eq!(section.content_size, 24);
/// assert_eq!(section.content, "This is a test section.");
///
/// ```
pub fn read_section_unsafe(dat_file: &mut DATFile) -> Result<Section, DATError> {
    // Read section header.
    let mut sec_header_bytes = [0u8; SECTION_HEADER_SIZE];
    // Manually wrap EOF into DATError EOF
    match dat_file.read_exact(&mut sec_header_bytes) {
        Ok(_) => (),
        Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Err(DATError::EndOfFile("Found EOF looking for next section."))
        }
        Err(err) => return Err(DATError::from(err)),
    };
    let (tag, content_size) = get_section_header_contents(&sec_header_bytes)?;
    // Read section content
    let mut sec_content_bytes = vec![0u8; usize::from(content_size - 1)];
    match dat_file.read_exact(&mut sec_content_bytes) {
        Ok(_) => (),
        Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Err(DATError::EndOfFile("Found EOF reading next section."))
        }
        Err(err) => return Err(DATError::from(err)),
    };
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
/// but returns a `Vec<Section>` instead of raw bytes. This does not check that the
/// file is of a type that should contain sections.
///
/// # Errors
///
/// Returns a [`DATError::Overflow`](crate::dat_error::DATError::Overflow) or
/// [`DATError::Underflow`](crate::dat_error::DATError::Underflow) if a section content block
/// does not match the expected length specified in the section header.
///
/// Returns a [`DATError::BadEncoding`](crate::dat_error::DATError::BadEncoding) if a section does not
/// contain valid utf8 text.
///
/// Returns a [`DATError::BadHeader`](crate::dat_error::DATError::BadHeader) if the specified file does not
/// have a valid DAT header.
///
/// If an I/O error occurs while reading the file, a [`DATError::FileIO`](crate::dat_error::DATError::FileIO)
/// error will be returned wrapping the underlying FS error.
///
/// # Examples
/// ```rust
/// use libxivdat::section::read_section_content_unsafe;
///
/// let section = read_section_content_unsafe("./resources/TEST_SECTION.DAT").unwrap();
///
/// assert_eq!(section[0].tag, "T");
/// assert_eq!(section[0].content_size, 24);
/// assert_eq!(section[0].content, "This is a test section.");
///
/// assert_eq!(section[1].tag, "A");
/// assert_eq!(section[1].content_size, 22);
/// assert_eq!(section[1].content, "Another test section.");
/// ```
pub fn read_section_content_unsafe<P: AsRef<Path>>(path: P) -> Result<Vec<Section>, DATError> {
    let content_bytes = read_content(path)?;
    let section_data = as_section_vec(&content_bytes)?;
    Ok(section_data.iter().map(Section::from).collect())
}

// --- Unit Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat_file::{read_content, DATFile};

    const TEST_FILE_PATH: &str = "./resources/TEST_SECTION.DAT";
    const TEST_NON_SECTION_PATH: &str = "./resources/TEST_BLOCK.DAT";
    const TEST_FILE_SEC1_CONTENTS: (&str, u16, &str) = ("T", 24, "This is a test section.");
    const TEST_FILE_SEC2_CONTENTS: (&str, u16, &str) = ("A", 22, "Another test section.");
    const TEST_SEC: [u8; 7] = [0x41, 0x04, 0x00, 0x41, 0x42, 0x43, 0x00];
    const TEST_SEC_CONTENTS: (&str, u16, &str) = ("A", 4, "ABC");
    const TEST_SEC_NOT_UTF8: [u8; 7] = [0xc2, 0x04, 0x00, 0x01, 0x01, 0x01, 0x00];
    const TEST_SEC_TOO_SHORT: [u8; 6] = [0x41, 0x04, 0x00, 0x41, 0x42, 0x00];
    const TEST_SEC_TOO_LONG: [u8; 8] = [0x41, 0x04, 0x00, 0x41, 0x42, 0x43, 0x44, 0x00];

    // --- Module Functions

    #[test]
    fn test_as_section() -> Result<(), String> {
        match as_section(&TEST_SEC[..]) {
            Ok(section) => {
                assert_eq!(section.tag, TEST_SEC_CONTENTS.0);
                assert_eq!(section.content_size, TEST_SEC_CONTENTS.1);
                assert_eq!(section.content, TEST_SEC_CONTENTS.2);
                Ok(())
            }
            Err(err) => Err(format!("Error: {}", err)),
        }
    }

    #[test]
    fn test_as_section_vec() -> Result<(), String> {
        let sec_bytes = match read_content(TEST_FILE_PATH) {
            Ok(sec_bytes) => sec_bytes,
            Err(err) => return Err(format!("Error reading file: {}", err)),
        };
        match as_section_vec(&sec_bytes) {
            Ok(section) => {
                assert_eq!(section[0].tag, TEST_FILE_SEC1_CONTENTS.0);
                assert_eq!(section[0].content_size, TEST_FILE_SEC1_CONTENTS.1);
                assert_eq!(section[0].content, TEST_FILE_SEC1_CONTENTS.2);
                assert_eq!(section[1].tag, TEST_FILE_SEC2_CONTENTS.0);
                assert_eq!(section[1].content_size, TEST_FILE_SEC2_CONTENTS.1);
                assert_eq!(section[1].content, TEST_FILE_SEC2_CONTENTS.2);
                Ok(())
            }
            Err(err) => Err(format!("Error: {}", err)),
        }
    }

    #[test]
    fn test_as_section_vec_error_overflow() -> Result<(), String> {
        let mut sec_bytes = match read_content(TEST_FILE_PATH) {
            Ok(sec_bytes) => sec_bytes,
            Err(err) => return Err(format!("Error reading file: {}", err)),
        };
        // Remove the terminating null from the first section and add in arbitrary data.
        sec_bytes[SECTION_HEADER_SIZE + usize::from(TEST_FILE_SEC1_CONTENTS.1) - 1] = 0x41;
        match as_section_vec(&sec_bytes) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err {
                DATError::Overflow(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_as_section_vec_error_underflow() -> Result<(), String> {
        let mut sec_bytes = match read_content(TEST_FILE_PATH) {
            Ok(sec_bytes) => sec_bytes,
            Err(err) => return Err(format!("Error reading file: {}", err)),
        };
        // Add an early terminating null to the first section.
        sec_bytes[SECTION_HEADER_SIZE + usize::from(TEST_FILE_SEC1_CONTENTS.1) - 2] = 0x00;
        match as_section_vec(&sec_bytes) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err {
                DATError::Underflow(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_as_section_vec_error_encoding() -> Result<(), String> {
        let mut sec_bytes = match read_content(TEST_FILE_PATH) {
            Ok(sec_bytes) => sec_bytes,
            Err(err) => return Err(format!("Error reading file: {}", err)),
        };
        // Add a random utf8 control char.
        sec_bytes[SECTION_HEADER_SIZE + usize::from(TEST_FILE_SEC1_CONTENTS.1) - 2] = 0xc2;
        match as_section_vec(&sec_bytes) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err {
                DATError::BadEncoding(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_read_section() -> Result<(), String> {
        let mut dat_file = match DATFile::open(TEST_FILE_PATH) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Error opening file: {}", err)),
        };
        match read_section(&mut dat_file) {
            Ok(section) => {
                assert_eq!(section.tag, TEST_FILE_SEC1_CONTENTS.0);
                assert_eq!(section.content_size, TEST_FILE_SEC1_CONTENTS.1);
                assert_eq!(section.content, TEST_FILE_SEC1_CONTENTS.2);
                Ok(())
            }
            Err(err) => Err(format!("Error: {}", err)),
        }
    }

    #[test]
    fn test_read_section_error_type() -> Result<(), String> {
        let mut dat_file = match DATFile::open(TEST_NON_SECTION_PATH) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Error opening file: {}", err)),
        };
        match read_section(&mut dat_file) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err {
                DATError::IncorrectType(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_read_section_error_eof() -> Result<(), String> {
        let mut dat_file = match DATFile::open(TEST_FILE_PATH) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Error opening file: {}", err)),
        };
        dat_file.seek(SeekFrom::End(-1)).unwrap();
        match read_section(&mut dat_file) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err {
                DATError::EndOfFile(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_read_section_content() -> Result<(), String> {
        // Errors are indirectly tested by as_section_vec tests.
        match read_section_content(TEST_FILE_PATH) {
            Ok(section) => {
                assert_eq!(section[0].tag, TEST_FILE_SEC1_CONTENTS.0);
                assert_eq!(section[0].content_size, TEST_FILE_SEC1_CONTENTS.1);
                assert_eq!(section[0].content, TEST_FILE_SEC1_CONTENTS.2);
                assert_eq!(section[1].tag, TEST_FILE_SEC2_CONTENTS.0);
                assert_eq!(section[1].content_size, TEST_FILE_SEC2_CONTENTS.1);
                assert_eq!(section[1].content, TEST_FILE_SEC2_CONTENTS.2);
                Ok(())
            }
            Err(err) => Err(format!("Error: {}", err)),
        }
    }

    #[test]
    fn test_read_section_content_error_type() -> Result<(), String> {
        match read_section_content(TEST_NON_SECTION_PATH) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err {
                DATError::IncorrectType(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    // --- Section

    #[test]
    fn test_section_new() -> Result<(), String> {
        match Section::new("T".to_string(), "Test".to_string()) {
            Ok(section) => {
                assert_eq!(section.tag, "T");
                assert_eq!(section.content_size, 5);
                assert_eq!(section.content, "Test");
                Ok(())
            }
            Err(err) => Err(format!("Error: {}", err)),
        }
    }

    #[test]
    fn test_section_new_error_title_size() -> Result<(), String> {
        match Section::new("Too long".to_string(), "Test".to_string()) {
            Ok(_) => Err("No error returned".to_owned()),
            Err(err) => match err {
                DATError::InvalidInput(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_section_new_error_content_size() -> Result<(), String> {
        match Section::new("T".to_string(), (0..u16::MAX).map(|_| "X").collect()) {
            Ok(_) => Err("No error returned".to_owned()),
            Err(err) => match err {
                DATError::Overflow(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_section_from_bytes() -> Result<(), String> {
        match Section::try_from(&TEST_SEC[..]) {
            Ok(section) => {
                assert_eq!(section.tag, TEST_SEC_CONTENTS.0);
                assert_eq!(section.content_size, TEST_SEC_CONTENTS.1);
                assert_eq!(section.content, TEST_SEC_CONTENTS.2);
                Ok(())
            }
            Err(err) => Err(format!("Error: {}", err)),
        }
    }

    #[test]
    fn test_section_from_bytes_error_overflow() -> Result<(), String> {
        match Section::try_from(&TEST_SEC_TOO_SHORT[..]) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err {
                DATError::Overflow(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_section_from_bytes_error_underflow() -> Result<(), String> {
        match Section::try_from(&TEST_SEC_TOO_LONG[..]) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err {
                DATError::Underflow(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_section_from_bytes_error_encoding() -> Result<(), String> {
        match Section::try_from(&TEST_SEC_NOT_UTF8[..]) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err {
                DATError::BadEncoding(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_section_to_bytes() -> Result<(), String> {
        let sec_bytes = Vec::<u8>::from(Section {
            tag: TEST_SEC_CONTENTS.0.to_owned(),
            content_size: TEST_SEC_CONTENTS.1,
            content: TEST_SEC_CONTENTS.2.to_owned(),
        });
        Ok(assert_eq!(sec_bytes, TEST_SEC))
    }

    #[test]
    fn test_section_to_sectiondata() -> Result<(), String> {
        let test_sec = Section {
            tag: TEST_SEC_CONTENTS.0.to_owned(),
            content_size: TEST_SEC_CONTENTS.1,
            content: TEST_SEC_CONTENTS.2.to_owned(),
        };
        let sec_data = SectionData::from(&test_sec);
        assert_eq!(sec_data.tag, TEST_SEC_CONTENTS.0);
        assert_eq!(sec_data.content_size, TEST_SEC_CONTENTS.1);
        assert_eq!(sec_data.content, TEST_SEC_CONTENTS.2);
        Ok(())
    }

    // --- SectionData

    #[test]
    fn test_sectiondata_new() -> Result<(), String> {
        match SectionData::new("T", "Test") {
            Ok(section) => {
                assert_eq!(section.tag, "T");
                assert_eq!(section.content_size, 5);
                assert_eq!(section.content, "Test");
                Ok(())
            }
            Err(err) => Err(format!("Error: {}", err)),
        }
    }

    #[test]
    fn test_sectiondata_new_error_title_size() -> Result<(), String> {
        match SectionData::new("Too long", "Test") {
            Ok(_) => Err("No error returned".to_owned()),
            Err(err) => match err {
                DATError::InvalidInput(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_sectiondata_new_error_content_size() -> Result<(), String> {
        match SectionData::new("T", &(0..u16::MAX).map(|_| "X").collect::<String>()) {
            Ok(_) => Err("No error returned".to_owned()),
            Err(err) => match err {
                DATError::Overflow(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_sectiondata_from_bytes() -> Result<(), String> {
        match SectionData::try_from(&TEST_SEC[..]) {
            Ok(section) => {
                assert_eq!(section.tag, TEST_SEC_CONTENTS.0);
                assert_eq!(section.content_size, TEST_SEC_CONTENTS.1);
                assert_eq!(section.content, TEST_SEC_CONTENTS.2);
                Ok(())
            }
            Err(err) => Err(format!("Error: {}", err)),
        }
    }

    #[test]
    fn test_sectiondata_from_bytes_error_overflow() -> Result<(), String> {
        match SectionData::try_from(&TEST_SEC_TOO_SHORT[..]) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err {
                DATError::Overflow(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_sectiondata_from_bytes_error_underflow() -> Result<(), String> {
        match SectionData::try_from(&TEST_SEC_TOO_LONG[..]) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err {
                DATError::Underflow(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_sectiondata_from_bytes_error_encoding() -> Result<(), String> {
        match SectionData::try_from(&TEST_SEC_NOT_UTF8[..]) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err {
                DATError::BadEncoding(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_sectiondata_to_bytes() -> Result<(), String> {
        let sec_bytes = Vec::<u8>::from(SectionData {
            tag: TEST_SEC_CONTENTS.0,
            content_size: TEST_SEC_CONTENTS.1,
            content: TEST_SEC_CONTENTS.2,
        });
        Ok(assert_eq!(sec_bytes, TEST_SEC))
    }

    #[test]
    fn test_sectiondata_to_section() -> Result<(), String> {
        let test_sec = SectionData {
            tag: TEST_SEC_CONTENTS.0,
            content_size: TEST_SEC_CONTENTS.1,
            content: TEST_SEC_CONTENTS.2,
        };
        let section = Section::from(&test_sec);
        assert_eq!(section.tag, TEST_SEC_CONTENTS.0);
        assert_eq!(section.content_size, TEST_SEC_CONTENTS.1);
        assert_eq!(section.content, TEST_SEC_CONTENTS.2);
        Ok(())
    }
}
