use std::convert::{TryFrom, TryInto};
use std::fs::{File, Metadata, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::dat_error::DATError;
use crate::dat_type::*;

/// Header size in bytes.
pub const HEADER_SIZE: u32 = 0x11;
/// Offset of the max_size header value from the actual file size on disk.
/// This value should be added to the `max_size` in the header to produce size of the file on disk.
/// Files have a null-padded "footer" of 15 bytes that cannot be omitted, as well as the 17 byte header.
const MAX_SIZE_OFFSET: u32 = 32;
/// Index of the `file_type` header record.
const INDEX_FILE_TYPE: usize = 0x00;
/// Index of the `max_size` header record.
const INDEX_MAX_SIZE: usize = 0x04;
/// Index of the `content_size` header record.
const INDEX_CONTENT_SIZE: usize = 0x08;

/// A reference to an open DAT file on the system. This emulates the standard lib
/// [`std::fs::File`] but provides additional DAT-specific functionality.
///
/// Reads and writes to DAT files are performed only on the data contents of the file.
/// XOR masks are automatically applied as necessary.
///
/// # Examples
/// ```rust
/// use libxivdat::dat_file::DATFile;
/// use libxivdat::dat_type::DATType;
/// use std::io::Read;
///
/// let mut dat_file = match DATFile::open("./resources/TEST_XOR.DAT") {
///     Ok(dat_file) => dat_file,
///     Err(_) => panic!("Something broke!")
/// };
///
/// match dat_file.file_type() {
///     DATType::Macro => {
///         let mut macro_bytes = vec![0u8; dat_file.content_size() as usize - 1];
///         match dat_file.read(&mut macro_bytes) {
///             Ok(count) => println!("Read {} bytes.", count),
///             Err(_) => panic!("Reading broke!")
///         }
///     },
///     _ => panic!("Not a macro file!")
/// };
/// ```
#[derive(Debug)]
pub struct DATFile {
    /// Size in bytes of the readable content of the DAT file. This size includes a trailing null byte.
    /// The size of readable content is 1 less than this value.
    content_size: u32,
    /// Type of the file. This will be inferred from the header when converting directly from a `File`.
    file_type: DATType,
    /// A single byte that marks the end of the header. This is `0xFF` for most DAT files, but occasionally varies.
    /// The purpose of this byte is unknown.
    header_end_byte: u8,
    /// Maximum allowed size of the content in bytes. The writeable size is 1 byte less than this value.
    /// Excess available space not used by content is null padded.
    ///
    /// Altering this value from the defaults provided for each file type may
    /// produce undefined behavior in the game client.
    max_size: u32,
    /// The underlying [`std::fs::File`].
    raw_file: File,
}

impl Read for DATFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Limit read size to content region of the DAT file.
        let cur_pos = self.stream_position()? as u32;
        let max_end = self.content_size - 1;
        let read_end = match u32::try_from(buf.len()) {
            Ok(safe_buf_len) if cur_pos + safe_buf_len < max_end => cur_pos + safe_buf_len,
            // Maximum read extent should be the last byte of content (excluding the terminating null).
            _ => max_end,
        };
        // read_len can never be larger than the input buffer len
        let read_len = (read_end - cur_pos) as usize;
        if read_len < 1 {
            return Ok(0);
        }
        // Initialize a temporary buffer used for applying masks
        let mut internal_buf = vec![0u8; read_len];
        let count = self.raw_file.read(&mut internal_buf)?;
        // Apply XOR mask to content data if needed.
        if let Some(mask_val) = get_mask_for_type(&self.file_type) {
            for byte in internal_buf.iter_mut() {
                *byte ^= mask_val;
            }
        }
        // Copy internal buffer into input buffer.
        buf[..read_len].clone_from_slice(&internal_buf);
        Ok(count)
    }
}

impl Seek for DATFile {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, std::io::Error> {
        let cursor = match pos {
            // Match `File` behavior of complaining if cursor goes negative relative to start.
            SeekFrom::Current(offset) => {
                if self.raw_file.stream_position()? as i64 + offset < HEADER_SIZE as i64 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Invalid argument",
                    ));
                } else {
                    self.raw_file.seek(pos)?
                }
            }
            // Treat content end as EOF and seek backwards from there
            SeekFrom::End(offset) => self.raw_file.seek(SeekFrom::End(
                offset
                    - (self.max_size as i64 - self.content_size as i64)
                    - (MAX_SIZE_OFFSET as i64 - HEADER_SIZE as i64)
                    - 1,
            ))?,
            // Just offset the seek, treating first content byte as 0.
            SeekFrom::Start(offset) => self.raw_file.seek(SeekFrom::Start(HEADER_SIZE as u64 + offset))?,
        };
        Ok(cursor - HEADER_SIZE as u64)
    }
}

impl Write for DATFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Get current cursor position for length checking.
        let content_cursor = self.stream_position()? as u32;

        // A buf longer than u32 max is always too long
        let buf_len = match u32::try_from(buf.len()) {
            Ok(len) => len,
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    DATError::Overflow("Content too long to write."),
                ))
            }
        };

        // Update content size if necessary
        // A content size > u32 max is always too long.
        match content_cursor.checked_add(buf_len + 1) {
            Some(new_content_size) if new_content_size > self.content_size => {
                // A content size > max size is too long
                if new_content_size > self.max_size {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        DATError::Overflow("Content size would exdeed maximum size after write."),
                    ));
                }
                // Write the new content size
                self.write_content_size_header(new_content_size)?;
            }
            Some(_) => (),
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    DATError::Overflow("Content size would exceed maximum possible size (u32::MAX) after write."),
                ))
            }
        };

        // Copy write buffer and apply XOR mask if needed.
        match get_mask_for_type(&self.file_type) {
            Some(mask_val) => {
                let mut masked_bytes = vec![0u8; buf.len()];
                masked_bytes.copy_from_slice(buf);
                for byte in masked_bytes.iter_mut() {
                    *byte ^= mask_val;
                }
                Ok(self.raw_file.write(&masked_bytes)?)
            }
            None => Ok(self.raw_file.write(buf)?),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.raw_file.flush()
    }
}

impl DATFile {
    /// Returns the size of the current content contained in the DAT file.
    /// DAT files store content as a null-terminated CString, so this size
    /// is one byte larger than the actual content.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::dat_file::DATFile;
    ///
    /// let mut dat_file = DATFile::open("./resources/TEST.DAT").unwrap();
    /// let content_size = dat_file.content_size();
    /// ```
    pub fn content_size(&self) -> u32 {
        self.content_size
    }

    /// Creates a new DAT file with an empty content block in read/write mode.
    /// This will truncate an existing file if one exists at the specified path.
    ///
    /// By default, this will use the default max size for the specified type from
    /// [`get_default_max_size_for_type()`](crate::dat_type::get_default_max_size_for_type()) and
    /// default header ending from [`get_default_end_byte_for_type()`](crate::dat_type::get_default_end_byte_for_type()).
    /// To cicumvent this behavior, you can use [`create_unsafe()`](Self::create_unsafe()`). Note
    /// that DAT files with nonstandard sizes may produce undefined behavior in the game client.
    ///
    /// # Errors
    ///
    /// If an I/O error creating the file occurs, a [`DATError::FileIO`](crate::dat_error::DATError::FileIO)
    /// error will be returned wrapping the underlying FS error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::dat_file::DATFile;
    /// use libxivdat::dat_type::DATType;
    /// # extern crate tempfile;
    /// # use tempfile::tempdir;
    /// # let temp_dir = tempdir().unwrap();
    /// # let path = temp_dir.path().join("TEST.DAT");
    ///
    /// let mut dat_file = DATFile::create(&path, DATType::Macro);
    /// ```
    pub fn create<P: AsRef<Path>>(path: P, dat_type: DATType) -> Result<Self, DATError> {
        let max_size = get_default_max_size_for_type(&dat_type).unwrap_or(0);
        let end_byte = get_default_end_byte_for_type(&dat_type).unwrap_or(0);
        Self::create_unsafe(path, dat_type, 1, max_size, end_byte)
    }

    /// Creates a new DAT file with a null-padded content bock of the specifed size in read/write mode.
    /// This will truncate an existing file if one exists at the specified path.
    ///
    /// This function allows a custom, not-necessarily-valid maximum length and end byte to be set. Note
    /// that DAT files with nonstandard sizes and headers may produce undefined behavior in the game client.
    ///
    /// # Errors
    ///
    /// If an I/O error creating the file occurs, a [`DATError::FileIO`](crate::dat_error::DATError::FileIO)
    /// error will be returned wrapping the underlying FS error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::dat_file::DATFile;
    /// use libxivdat::dat_type::DATType;
    ///
    /// # extern crate tempfile;
    /// # use tempfile::tempdir;
    /// # let temp_dir = tempdir().unwrap();
    /// # let path = temp_dir.path().join("TEST.DAT");
    ///
    /// // Create an empty (content length 1) macro file with a custom max size and end byte. This probably isn't valid.
    /// let mut dat_file = DATFile::create_unsafe(&path, DATType::Macro, 1, 1024, 0x01);
    /// ```
    pub fn create_unsafe<P: AsRef<Path>>(
        path: P, dat_type: DATType, content_size: u32, max_size: u32, end_byte: u8,
    ) -> Result<Self, DATError> {
        // Create a minimal content size 0 DAT file, then reopen it as a DATFile.
        {
            let mut raw_file = File::create(&path)?;
            raw_file.set_len((max_size + MAX_SIZE_OFFSET) as u64)?;
            // Write header type
            raw_file.seek(SeekFrom::Start(INDEX_FILE_TYPE as u64))?;
            raw_file.write_all(&(dat_type as i32).to_le_bytes())?;
            // Write header max_size
            raw_file.seek(SeekFrom::Start(INDEX_MAX_SIZE as u64))?;
            raw_file.write_all(&max_size.to_le_bytes())?;
            // Write a content size header of 1 (content size 0 is invalid).
            // Real content size is set below and padded apprioriately.
            raw_file.seek(SeekFrom::Start(INDEX_CONTENT_SIZE as u64))?;
            raw_file.write_all(&1u32.to_le_bytes())?;
            // End header
            raw_file.seek(SeekFrom::Start(HEADER_SIZE as u64 - 1))?;
            raw_file.write_all(&[end_byte])?;
        }
        let mut dat_file = DATFile::open_options(path, OpenOptions::new().read(true).write(true).create(true))?;
        // Write the content block and content_size header.
        dat_file.set_content_size(content_size)?;
        Ok(dat_file)
    }

    /// Creates a new DAT file with a specific content block in read/write mode.
    /// This will truncate an existing file if one exists at the specified path.
    ///
    /// This is shorthand for creating the DAT file, then calling
    /// [`write()`](Self::write()). This function also resets the cursor to
    /// the beginning of the content block after writing.
    ///
    /// By default, this will use the default max size for the specified type from
    /// [`get_default_max_size_for_type()`](crate::dat_type::get_default_max_size_for_type()) and
    /// default header ending from [`get_default_end_byte_for_type()`](crate::dat_type::get_default_end_byte_for_type()).
    /// To cicumvent this behavior, you can use [`create_unsafe()`](Self::create_unsafe()`). Note
    /// that DAT files with nonstandard sizes may produce undefined behavior in the game client.
    ///
    /// # Errors
    ///
    /// If an I/O error creating the file occurs, a [`DATError::FileIO`](crate::dat_error::DATError::FileIO)
    /// error will be returned wrapping the underlying FS error.
    ///
    /// A [`DATError::Overflow`](crate::dat_error::DATError::Overflow) is returned
    /// if the provided content size is too large, or if the content size exceeds the maximum size.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::dat_file::DATFile;
    /// use libxivdat::dat_type::DATType;
    ///
    /// # extern crate tempfile;
    /// # use tempfile::tempdir;
    /// # let temp_dir = tempdir().unwrap();
    /// # let path = temp_dir.path().join("TEST.DAT");
    ///
    /// let mut dat_file = DATFile::create_with_content(&path, DATType::Macro, b"Not really a macro.");
    /// ```
    pub fn create_with_content<P: AsRef<Path>>(path: P, dat_type: DATType, content: &[u8]) -> Result<Self, DATError> {
        let max_size = get_default_max_size_for_type(&dat_type).unwrap_or(0);
        let end_byte = get_default_end_byte_for_type(&dat_type).unwrap_or(0);
        let mut dat_file = Self::create_unsafe(path, dat_type, 1, max_size, end_byte)?;
        dat_file.write_all(&content)?;
        dat_file.seek(SeekFrom::Start(0))?;
        Ok(dat_file)
    }

    /// Returns the file type of the DAT file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::dat_file::DATFile;
    /// use libxivdat::dat_type::DATType;
    ///
    /// let mut dat_file = DATFile::open("./resources/TEST_XOR.DAT").unwrap();
    /// match dat_file.file_type() {
    ///     DATType::Macro => println!("Macro file!"),
    ///     _ => panic!("Nope!")
    /// }
    /// ```
    pub fn file_type(&self) -> DATType {
        self.file_type
    }

    /// Returns the terminating byte of the DAT file's
    /// header. The purpose of this byte is unknown,
    /// but it is almost always 0xFF.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::dat_file::DATFile;
    ///
    /// let mut dat_file = DATFile::open("./resources/TEST_XOR.DAT").unwrap();
    /// let header_end_byte = dat_file.header_end_byte();
    /// ```
    pub fn header_end_byte(&self) -> u8 {
        self.header_end_byte
    }

    /// Returns the maximum size allowed for the content block
    /// of the DAT file. Content is stored as a null-terminated CString,
    /// so the actual maximum allowed content is 1 byte less than `max_size`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::dat_file::DATFile;
    ///
    /// let mut dat_file = DATFile::open("./resources/TEST_XOR.DAT").unwrap();
    /// let header_end_byte = dat_file.max_size();
    /// ```
    pub fn max_size(&self) -> u32 {
        self.max_size
    }

    /// Calls [`metadata()`](std::fs::File::sync_all()) on the underlying [`std::fs::File`].
    ///
    /// # Errors
    ///
    /// This function will return any underling I/O errors as a
    /// [`DATError::FileIO`](crate::dat_error::DATError::FileIO).
    pub fn metadata(&self) -> Result<Metadata, DATError> {
        Ok(self.raw_file.metadata()?)
    }

    /// Attempts to open a DAT file in read-only mode.
    /// To set different file access with [`OpenOptions`](std::fs::OpenOptions),
    /// use [`open_options()`](Self::open_options())
    ///
    /// # Errors
    ///
    /// If an I/O error opening the file occurs, a [`DATError::FileIO`](crate::dat_error::DATError::FileIO)
    /// error will be returned wrapping the underlying FS error.
    ///
    /// A [`DATError::BadHeader`](crate::dat_error::DATError::BadHeader) will be returned if the header
    /// cannot be validated, indicating a non-DAT or corrupt file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::dat_file::DATFile;
    ///
    /// let mut dat_file = DATFile::open("./resources/TEST.DAT");
    /// ```
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, DATError> {
        let mut raw_file = File::open(path)?;
        let mut header_bytes = [0u8; HEADER_SIZE as usize];
        raw_file.read_exact(&mut header_bytes)?;
        let (file_type, max_size, content_size, header_end_byte) = get_header_contents(&header_bytes)?;
        Ok(DATFile {
            content_size,
            file_type,
            header_end_byte,
            max_size,
            raw_file,
        })
    }

    /// Attempts to open a DAT file using an [`OpenOptions`](std::fs::OpenOptions) builder.
    /// A reference to the `OpenOptions` struct itself should be passed in, not the `File` it opens.
    /// Do not end the options chain with `open("foo.txt")` as with opening a standard file.
    ///
    /// # Errors
    ///
    /// If an I/O error opening the file occurs, a [`DATError::FileIO`](crate::dat_error::DATError::FileIO)
    /// error will be returned wrapping the underlying FS error.
    ///
    /// A [`DATError::BadHeader`](crate::dat_error::DATError::BadHeader) will be returned if the header
    /// cannot be validated, indicating a non-DAT or corrupt file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::dat_file::DATFile;
    /// use std::fs::OpenOptions;
    ///
    /// let mut open_opts = OpenOptions::new();
    /// open_opts.read(true).write(true);
    /// let mut dat_file = DATFile::open_options("./resources/TEST.DAT", &mut open_opts);
    /// ```
    pub fn open_options<P: AsRef<Path>>(path: P, options: &mut OpenOptions) -> Result<Self, DATError> {
        let mut raw_file = options.open(path)?;
        let mut header_bytes = [0u8; HEADER_SIZE as usize];
        raw_file.read_exact(&mut header_bytes)?;
        let (file_type, max_size, content_size, header_end_byte) = get_header_contents(&header_bytes)?;
        Ok(DATFile {
            content_size,
            file_type,
            header_end_byte,
            max_size,
            raw_file,
        })
    }

    /// Truncates or extends the readable content section of the DAT file.
    /// This emulates the behavior of [`std::fs::File::set_len()`], but only
    /// operates on the content region of the DAT file. Because DAT files store
    /// content as null-terminated CStrings, the actual writeable space will be
    /// one byte less than specified.
    ///
    /// # Errors
    ///
    /// This function will return any underling I/O errors as a
    /// [`DATError::FileIO`](crate::dat_error::DATError::FileIO).
    ///
    /// Additionally, it may return a [`DATError::Overflow`](crate::dat_error::DATError::Overflow)
    /// error if the new content size would exceed the maximum allowed size. This size may be adjusted using
    /// [`set_max_size()`](Self::set_max_size()), but modifying it may not produce a valid file for the game client.
    pub fn set_content_size(&mut self, new_size: u32) -> Result<(), DATError> {
        // Quick noop for no change
        if new_size == self.content_size {
            return Ok(());
        }
        // Check for valid size
        if new_size == 0 {
            return Err(DATError::InvalidInput("Content size must be > 0."));
        }
        if new_size > self.max_size {
            return Err(DATError::Overflow("Content size would exceed maximum size."));
        }
        // Save pre-run cursor.
        let pre_cursor = self.raw_file.seek(SeekFrom::Current(0))?;
        // For shrinks, fill with actual null bytes starting at new content end.
        // For grows, pad with the the content mask byte (null ^ mask) starting at old content end to new end.
        let (padding_byte, write_size) = if new_size > self.content_size {
            self.seek(SeekFrom::End(0))?;
            (
                get_mask_for_type(&self.file_type).unwrap_or(0),
                new_size - self.content_size,
            )
        } else {
            self.seek(SeekFrom::Start(new_size as u64))?;
            (0, self.content_size - new_size)
        };
        // Handle having to write in chunks for usize = 16.
        match usize::try_from(write_size) {
            Ok(safe_write_size) => {
                self.raw_file.write_all(&vec![padding_byte; safe_write_size])?;
            }
            Err(_) => {
                let mut remaining_bytes = write_size;
                loop {
                    match usize::try_from(remaining_bytes) {
                        Ok(safe_write_size) => {
                            self.raw_file.write_all(&vec![padding_byte; safe_write_size])?;
                            break;
                        }
                        Err(_) => {
                            self.raw_file.write_all(&vec![padding_byte; usize::MAX])?;
                            remaining_bytes -= usize::MAX as u32;
                        }
                    };
                }
            }
        }
        // Write the new content size to the header
        self.write_content_size_header(new_size)?;
        // Reset file cursor
        self.raw_file.seek(SeekFrom::Start(pre_cursor))?;
        Ok(())
    }

    /// Truncates or extends the full DAT file.
    /// This emulates the behavior of [`std::fs::File::set_len()`].
    /// Because DAT files store content as null-terminated CStrings,
    /// the actual useable space will be one byte less than specified.
    ///
    /// Files with a non-default maximum size may cause undefined behavior in the game client.
    ///
    /// # Errors
    ///
    /// This function will return any underling I/O errors as a
    /// [`DATError::FileIO`](crate::dat_error::DATError::FileIO).
    ///
    /// A [`DATError::Overflow`](crate::dat_error::DATError::Overflow) is returned
    /// if the maximum size would be shorter than the content size after shrinking. To correct this,
    /// first [`set_content_size()`](Self::set_content_size()).
    pub fn set_max_size(&mut self, new_size: u32) -> Result<(), DATError> {
        // Quick noop for no change
        if new_size == self.max_size {
            return Ok(());
        }
        if new_size == 0 {
            return Err(DATError::InvalidInput("Content size must be > 0."));
        }
        // Check for valid size
        if new_size < self.content_size {
            return Err(DATError::Overflow("Content size would exceed maximum size."));
        }
        // Safe to resize
        self.raw_file.set_len((new_size + MAX_SIZE_OFFSET) as u64)?;
        // Write new max len to header
        self.write_max_size_header(new_size)?;
        Ok(())
    }

    /// Calls [`sync_all()`](std::fs::File::sync_all()) on the underlying [`std::fs::File`].
    ///
    /// # Errors
    ///
    /// This function will return any underling I/O errors as a
    /// [`DATError::FileIO`](crate::dat_error::DATError::FileIO).
    pub fn sync_all(&self) -> Result<(), DATError> {
        Ok(self.raw_file.sync_all()?)
    }

    /// Calls [`sync_data()`](std::fs::File::sync_data()) on the underlying [`std::fs::File`].
    ///
    /// # Errors
    ///
    /// This function will return any underling I/O errors as a
    /// [`DATError::FileIO`](crate::dat_error::DATError::FileIO).
    pub fn sync_data(&self) -> Result<(), DATError> {
        Ok(self.raw_file.sync_data()?)
    }

    /// Writes a new content size value to the [`DATFile`](Self) header.
    /// This updates both the struct and the header of the file on disk.
    /// This does not modify the actual content of the file.
    ///
    /// This should be used to update the `content_size` after writes that alter it.
    ///
    /// # Errors
    ///
    /// May return a [`std::io::Error`] if one is returned by an underlying fs operation.
    fn write_content_size_header(&mut self, size: u32) -> Result<(), std::io::Error> {
        let pre_cursor = self.raw_file.seek(SeekFrom::Current(0))?;
        self.raw_file.seek(SeekFrom::Start(INDEX_CONTENT_SIZE as u64))?;
        self.raw_file.write_all(&size.to_le_bytes())?;
        self.raw_file.seek(SeekFrom::Start(pre_cursor))?;
        self.content_size = size;
        Ok(())
    }

    /// Writes a new max size value to the [`DATFile`](Self) header.
    /// This updates both the struct and the header of the file on disk.
    /// This does not modify the actual size of the file.
    ///
    /// This should be used to update the `max_size` after writes that alter it.
    ///
    /// # Errors
    ///
    /// May return a [`std::io::Error`] if one is returned by an underlying fs operation.
    fn write_max_size_header(&mut self, size: u32) -> Result<(), std::io::Error> {
        let pre_cursor = self.raw_file.seek(SeekFrom::Current(0))?;
        self.raw_file.seek(SeekFrom::Start(INDEX_MAX_SIZE as u64))?;
        self.raw_file.write_all(&size.to_le_bytes())?;
        self.raw_file.seek(SeekFrom::Start(pre_cursor))?;
        self.max_size = size;
        Ok(())
    }
}

/// Checks the [`DATType`] of a DAT file based on the header contents. This should be treated as a best guess,
/// since the header is not fully understood.
///
/// # Errors
///
/// If an I/O error occurs while reading the file, a [`DATError::FileIO`](crate::dat_error::DATError::FileIO)
/// error will be returned wrapping the underlying FS error.
///
/// A [`DATError::BadHeader`](crate::dat_error::DATError::BadHeader) will be returned if the file header
/// cannot be validated, indicating a non-DAT or corrupt file.
///
/// # Examples
///
/// ```rust
/// use libxivdat::dat_file::check_type;
/// use libxivdat::dat_type::DATType;
///
/// let dat_type = check_type("./resources/TEST_XOR.DAT").unwrap();
/// assert_eq!(dat_type, DATType::Macro);
/// ```
pub fn check_type<P: AsRef<Path>>(path: P) -> Result<DATType, DATError> {
    let dat_file = DATFile::open(path)?;
    Ok(dat_file.file_type())
}

/// Tries to read an 0x11 length byte array as a DAT file header.
/// Returns a tuple containing (`file_type`, `max_size`, `content_size`, `header_end_byte`).
///
/// File type is inferred using known static values in the header, but the actual purpose of these bytes
/// is unknown. Inferred type should be treated as a best guess.
///
/// # Errors
/// This function will return a [`DATError::BadHeader`](crate::dat_error::DATError::BadHeader) if the data is not a valid header.
///
/// # Examples
/// ```rust
/// use libxivdat::dat_file::{get_header_contents, HEADER_SIZE};
/// use std::fs::File;
/// use std::io::Read;
///
/// let mut header_bytes = [0u8; HEADER_SIZE as usize];
/// let mut file = File::open("./resources/TEST.DAT").unwrap();
/// file.read(&mut header_bytes).unwrap();
/// let (file_type, max_size, content_size, end_byte) = get_header_contents(&mut header_bytes).unwrap();
/// ```
///
/// # Data Structure
/// ```text
/// 0                                               1
/// 0  1  2  3  4  5  6  7  8  9  a  b  c  d  e  f  0  
/// |-+-++-+-|  |-+-++-+-|  |-+-++-+-|  |-+-++-+-|  |
/// |           |           |           |           \_ u8 header_end_byte
/// |           |           |           |              0xFF for all ^0x73 files, unique static values for ^0x31
/// |           |           |           \_ null
/// |           |           |              reserved?
/// |           |           \_ u32le content_size
/// |           |              (includes terminating null byte)
/// |           \_ u32le max_size
/// |              max content_size allowed; size on disk - 32 -> 17 byte header + minimum 15-byte null pad footer
/// \_ u32le file_type
///    constant value(s) per file type; probably actually 2 distinct bytes -> always <byte null byte null>
/// ```
pub fn get_header_contents(header: &[u8; HEADER_SIZE as usize]) -> Result<(DATType, u32, u32, u8), DATError> {
    // If these fail, something is very wrong.
    let file_type_id = u32::from_le_bytes(header[INDEX_FILE_TYPE..INDEX_MAX_SIZE].try_into()?);
    let max_size = u32::from_le_bytes(header[INDEX_MAX_SIZE..INDEX_CONTENT_SIZE].try_into()?);
    let content_size = u32::from_le_bytes(header[INDEX_CONTENT_SIZE..INDEX_CONTENT_SIZE + 4].try_into()?);
    let end_byte = header[HEADER_SIZE as usize - 1];

    // Validate that file type id bytes are present.
    if 0xff00ff00 & file_type_id > 0 {
        return Err(DATError::BadHeader("File type ID bytes are absent."));
    }

    // Validate that sizes make sense.
    if content_size > max_size {
        return Err(DATError::BadHeader("Content size exceeds max size in header."));
    }

    Ok((DATType::from(file_type_id), max_size, content_size, end_byte))
}

/// Attempts to read the entire content block of a DAT file, returning a byte vector.
/// This is a convenience function similar to [`std::fs::read`] that automatically
/// handles opening and closing the underlying file.
///
/// # Errors
///
/// If an I/O error occurs while reading the file, a [`DATError::FileIO`](crate::dat_error::DATError::FileIO)
/// error will be returned wrapping the underlying FS error.
///
/// A [`DATError::BadHeader`](crate::dat_error::DATError::BadHeader) will be returned if the file header
/// cannot be validated, indicating a non-DAT or corrupt file.
///
/// A [`DATError::Overflow`](crate::dat_error::DATError::Overflow) is returned if the content
/// would exceed the maximum size specified in the header.
///
/// On 16-bit platforms, a [`DATError::Overflow`](crate::dat_error::DATError::Overflow) may be returned
/// if the content is too long to fit into a 16-bit vec. Content length can never exceed u32::MAX, so this error
/// is impossible on other platforms.
///
/// # Examples
///
/// ```rust
/// use libxivdat::dat_file::read_content;
///
/// let dat_bytes = read_content("./resources/TEST.DAT").unwrap();
/// ```
pub fn read_content<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, DATError> {
    let mut dat_file = DATFile::open(path)?;
    let safe_content_size = usize::try_from(dat_file.content_size - 1)?;
    let mut buf = vec![0u8; safe_content_size];
    dat_file.read_exact(&mut buf)?;
    Ok(buf)
}

/// Attempts to write an input buffer as the content block of a DAT File,
/// replacing the entire existing contents and returning the number of bytes written.
/// This is a convenience function that automatically handles opening and closing the underlying file.
///
/// This will only write to an existing DAT file. Use [`DATFile::create()`](crate::dat_file::DATFile::create())
/// to create a new DAT file.
///
/// # Errors
///
/// If an I/O error occurs while reading the file, a [`DATError::FileIO`](crate::dat_error::DATError::FileIO)
/// error will be returned wrapping the underlying FS error.
///
/// A [`DATError::BadHeader`](crate::dat_error::DATError::BadHeader) will be returned if the file header
/// cannot be validated, indicating a non-DAT or corrupt file.
///
/// A [`DATError::Overflow`](crate::dat_error::DATError::Overflow) is returned if the content
/// would exceed the maximum size specified in the header or the maximum possible size (u32::MAX).
///
/// # Examples
///
/// ```rust
/// use libxivdat::dat_file::write_content;
/// # use libxivdat::dat_file::DATFile;
/// # use libxivdat::dat_type::DATType;
///
/// # extern crate tempfile;
/// # use tempfile::tempdir;
/// # let temp_dir = tempdir().unwrap();
/// # let path = temp_dir.path().join("TEST.DAT");
/// # DATFile::create(&path, DATType::Macro).unwrap();
///
/// write_content(&path, b"Who's awesome? You're awesome!").unwrap();
/// ```
pub fn write_content<P: AsRef<Path>>(path: P, buf: &[u8]) -> Result<usize, DATError> {
    let mut dat_file = DATFile::open_options(path, OpenOptions::new().read(true).write(true))?;
    if let Ok(safe_content_size) = u32::try_from(buf.len() + 1) {
        if safe_content_size != dat_file.content_size() {
            dat_file.set_content_size(safe_content_size)?;
        }
        Ok(dat_file.write(&buf)?)
    } else {
        Err(DATError::Overflow(
            "Content size would exceed maximum possible size (u32::MAX).",
        ))
    }
}

// --- Unit Tests

#[cfg(test)]
mod tests {
    extern crate tempfile;
    use tempfile::tempdir;

    use super::*;
    use std::fs::copy;
    const TEST_PATH: &str = "./resources/TEST.DAT";
    const TEST_XOR_PATH: &str = "./resources/TEST_XOR.DAT";
    const TEST_EMPTY_PATH: &str = "./resources/TEST_EMPTY.DAT";
    const TEST_CONTENTS: &[u8; 5] = b"Boop!";
    const TEST_XOR_CONTENTS: &[u8; 6] = b"Macro!";

    // --- Module Functions

    #[test]
    fn test_check_type() -> Result<(), String> {
        match check_type(TEST_XOR_PATH) {
            Ok(dat_type) => Ok(assert_eq!(dat_type, DATType::Macro)),
            Err(err) => Err(format!("Read error: {}", err)),
        }
    }

    #[test]
    fn test_get_header_contents() -> Result<(), String> {
        let header_bytes = [
            0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF,
        ];
        let (dat_type, max_size, content_size, end_byte) = match get_header_contents(&header_bytes) {
            Ok(res) => (res.0, res.1, res.2, res.3),
            Err(err) => return Err(format!("{}", err)),
        };
        assert_eq!(dat_type, DATType::Unknown);
        assert_eq!(max_size, 2);
        assert_eq!(content_size, 2);
        assert_eq!(end_byte, 0xFF);
        Ok(())
    }

    #[test]
    fn test_get_header_contents_error_bad_type() -> Result<(), String> {
        let header_bytes = [
            0x00, 0x01, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF,
        ];
        match get_header_contents(&header_bytes) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err {
                DATError::BadHeader(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_get_header_contents_error_sizes() -> Result<(), String> {
        let header_bytes = [
            0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF,
        ];
        match get_header_contents(&header_bytes) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err {
                DATError::BadHeader(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_read_content() -> Result<(), String> {
        match read_content(TEST_PATH) {
            Ok(content_bytes) => Ok(assert_eq!(&content_bytes, TEST_CONTENTS)),
            Err(err) => Err(format!("Read error: {}", err)),
        }
    }

    #[test]
    fn test_read_content_with_mask() -> Result<(), String> {
        match read_content(TEST_XOR_PATH) {
            Ok(content_bytes) => Ok(assert_eq!(&content_bytes, TEST_XOR_CONTENTS)),
            Err(err) => Err(format!("Read error: {}", err)),
        }
    }

    #[test]
    fn test_write_content() -> Result<(), String> {
        // Make a tempfile
        let tmp_dir = match tempdir() {
            Ok(tmp_dir) => tmp_dir,
            Err(err) => return Err(format!("Error creating temp dir: {}", err)),
        };
        let tmp_path = tmp_dir.path().join("TEST.DAT");
        match copy(TEST_PATH, &tmp_path) {
            Ok(_) => (),
            Err(err) => return Err(format!("Could not create temp file for testing: {}", err)),
        };
        // Write content
        let new_content = b"Hi!";
        match write_content(&tmp_path, new_content) {
            Ok(_) => (),
            Err(err) => return Err(format!("Error writing content: {}", err)),
        };
        // Check content
        match read_content(&tmp_path) {
            Ok(content_bytes) => Ok(assert_eq!(&content_bytes, new_content)),
            Err(err) => Err(format!("Error reading file after write: {}", err)),
        }
    }

    // --- DATFile

    #[test]
    fn test_datfile_open() -> Result<(), String> {
        match DATFile::open(TEST_PATH) {
            Ok(dat_file) => {
                assert_eq!(dat_file.content_size(), 6);
                assert_eq!(dat_file.max_size, 7);
                assert_eq!(dat_file.header_end_byte(), 0xFF);
                assert_eq!(dat_file.file_type(), DATType::Unknown);
                Ok(())
            }
            Err(err) => Err(format!("{}", err)),
        }
    }

    #[test]
    fn test_datfile_open_detect_type() -> Result<(), String> {
        match DATFile::open(TEST_XOR_PATH) {
            Ok(dat_file) => {
                assert_eq!(dat_file.content_size(), 7);
                assert_eq!(dat_file.max_size, 8);
                assert_eq!(dat_file.header_end_byte(), 0xFF);
                assert_eq!(dat_file.file_type(), DATType::Macro);
                Ok(())
            }
            Err(err) => Err(format!("{}", err)),
        }
    }

    #[test]
    fn test_datfile_open_options() -> Result<(), String> {
        let mut opts = std::fs::OpenOptions::new();
        opts.read(true).write(true);
        match DATFile::open_options(TEST_PATH, &mut opts) {
            Ok(dat_file) => {
                assert_eq!(dat_file.content_size(), 6);
                assert_eq!(dat_file.max_size, 7);
                assert_eq!(dat_file.header_end_byte(), 0xFF);
                assert_eq!(dat_file.file_type(), DATType::Unknown);
                Ok(())
            }
            Err(err) => Err(format!("{}", err)),
        }
    }

    #[test]
    fn test_datfile_create() -> Result<(), String> {
        let tmp_dir = match tempdir() {
            Ok(tmp_dir) => tmp_dir,
            Err(err) => return Err(format!("Error creating temp dir: {}", err)),
        };
        let tmp_path = tmp_dir.path().join("TEST.DAT");
        match DATFile::create(&tmp_path, DATType::Macro) {
            Ok(dat_file) => {
                assert_eq!(dat_file.content_size(), 1);
                assert_eq!(
                    dat_file.max_size,
                    get_default_max_size_for_type(&DATType::Macro).unwrap()
                );
                assert_eq!(
                    dat_file.header_end_byte(),
                    get_default_end_byte_for_type(&DATType::Macro).unwrap()
                );
                assert_eq!(dat_file.file_type(), DATType::Macro);
                Ok(())
            }
            Err(err) => Err(format!("{}", err)),
        }
    }

    #[test]
    fn test_datfile_create_with_content() -> Result<(), String> {
        let tmp_dir = match tempdir() {
            Ok(tmp_dir) => tmp_dir,
            Err(err) => return Err(format!("Error creating temp dir: {}", err)),
        };
        let tmp_path = tmp_dir.path().join("TEST.DAT");
        let content = b"Content!";
        match DATFile::create_with_content(&tmp_path, DATType::Macro, content) {
            Ok(mut dat_file) => {
                assert_eq!(dat_file.content_size(), content.len() as u32 + 1);
                assert_eq!(
                    dat_file.max_size,
                    get_default_max_size_for_type(&DATType::Macro).unwrap()
                );
                assert_eq!(
                    dat_file.header_end_byte(),
                    get_default_end_byte_for_type(&DATType::Macro).unwrap()
                );
                assert_eq!(dat_file.file_type(), DATType::Macro);

                let mut buf = [0u8; 8];
                match dat_file.read(&mut buf) {
                    Ok(_) => (),
                    Err(err) => return Err(format!("Could not read back content: {}", err)),
                }
                assert_eq!(&buf, content);

                Ok(())
            }
            Err(err) => Err(format!("{}", err)),
        }
    }

    #[test]
    fn test_datfile_create_unsafe() -> Result<(), String> {
        let tmp_dir = match tempdir() {
            Ok(tmp_dir) => tmp_dir,
            Err(err) => return Err(format!("Error creating temp dir: {}", err)),
        };
        let tmp_path = tmp_dir.path().join("TEST.DAT");
        match DATFile::create_unsafe(&tmp_path, DATType::Macro, 256, 512, 0) {
            Ok(dat_file) => {
                assert_eq!(dat_file.content_size(), 256);
                assert_eq!(dat_file.max_size, 512);
                assert_eq!(dat_file.header_end_byte(), 0);
                assert_eq!(dat_file.file_type(), DATType::Macro);
                Ok(())
            }
            Err(err) => Err(format!("{}", err)),
        }
    }

    #[test]
    fn test_datfile_set_content_size_grow() -> Result<(), String> {
        let tmp_dir = match tempdir() {
            Ok(tmp_dir) => tmp_dir,
            Err(err) => return Err(format!("Error creating temp dir: {}", err)),
        };
        let tmp_path = tmp_dir.path().join("TEST.DAT");
        let mut dat_file = match DATFile::create_with_content(&tmp_path, DATType::Macro, &[34u8; 8]) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Error creating temp file: {}", err)),
        };
        match dat_file.set_content_size(17) {
            Ok(_) => {
                assert_eq!(dat_file.content_size(), 17);
                // New content space should be mask^null padded on disk.
                // This is output as null bytes via `read()`, which automatically applies masks.
                let mut buf = [0u8; 16];
                match dat_file.read(&mut buf) {
                    Ok(_) => (),
                    Err(err) => return Err(format!("Could not read back content: {}", err)),
                }
                assert_eq!(&buf[..8], &[34u8; 8]);
                assert_eq!(&buf[8..], &[0u8; 8]);
                Ok(())
            }
            Err(err) => return Err(format!("Error setting content size: {}", err)),
        }
    }

    #[test]
    fn test_datfile_set_content_size_shrink() -> Result<(), String> {
        let tmp_dir = match tempdir() {
            Ok(tmp_dir) => tmp_dir,
            Err(err) => return Err(format!("Error creating temp dir: {}", err)),
        };
        let tmp_path = tmp_dir.path().join("TEST.DAT");
        let mut dat_file = match DATFile::create_with_content(&tmp_path, DATType::Macro, &[34u8; 8]) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Error creating temp file: {}", err)),
        };
        match dat_file.set_content_size(5) {
            Ok(_) => {
                assert_eq!(dat_file.content_size(), 5);
                // Byte 4 should be treated as EOF; the rest of the buffer should remain untouched.
                let mut buf = [1u8; 8];
                match dat_file.read(&mut buf) {
                    Ok(_) => (),
                    Err(err) => return Err(format!("Could not read back content: {}", err)),
                }
                assert_eq!(&buf[..4], &[34u8; 4]);
                assert_eq!(&buf[4..], &[1u8; 4]);
                Ok(())
            }
            Err(err) => return Err(format!("Error setting content size: {}", err)),
        }
    }

    #[test]
    fn test_datfile_set_content_size_error_zero_size() -> Result<(), String> {
        let tmp_dir = match tempdir() {
            Ok(tmp_dir) => tmp_dir,
            Err(err) => return Err(format!("Error creating temp dir: {}", err)),
        };
        let tmp_path = tmp_dir.path().join("TEST.DAT");
        let mut dat_file = match DATFile::create_with_content(&tmp_path, DATType::Macro, &[34u8; 8]) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Error creating temp file: {}", err)),
        };
        match dat_file.set_content_size(0) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err {
                DATError::InvalidInput(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_datfile_set_content_size_error_over_max() -> Result<(), String> {
        let tmp_dir = match tempdir() {
            Ok(tmp_dir) => tmp_dir,
            Err(err) => return Err(format!("Error creating temp dir: {}", err)),
        };
        let tmp_path = tmp_dir.path().join("TEST.DAT");
        let mut dat_file = match DATFile::create_unsafe(&tmp_path, DATType::Unknown, 1, 4, 0) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Error creating temp file: {}", err)),
        };
        match dat_file.set_content_size(8) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err {
                DATError::Overflow(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_datfile_set_max_size_grow() -> Result<(), String> {
        let tmp_dir = match tempdir() {
            Ok(tmp_dir) => tmp_dir,
            Err(err) => return Err(format!("Error creating temp dir: {}", err)),
        };
        let tmp_path = tmp_dir.path().join("TEST.DAT");
        let mut dat_file = match DATFile::create_unsafe(&tmp_path, DATType::Macro, 4, 8, 0) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Error creating temp file: {}", err)),
        };
        match dat_file.set_max_size(16) {
            Ok(_) => {
                assert_eq!(dat_file.max_size(), 16);
                // File size on disk should equal max size + MAX_SIZE_OFFSET.
                let meta = match dat_file.metadata() {
                    Ok(meta) => meta,
                    Err(err) => return Err(format!("Could not read back content: {}", err)),
                };
                assert_eq!(meta.len(), 16 + MAX_SIZE_OFFSET as u64);
                Ok(())
            }
            Err(err) => return Err(format!("Error setting content size: {}", err)),
        }
    }

    #[test]
    fn test_datfile_set_max_size_shrink() -> Result<(), String> {
        let tmp_dir = match tempdir() {
            Ok(tmp_dir) => tmp_dir,
            Err(err) => return Err(format!("Error creating temp dir: {}", err)),
        };
        let tmp_path = tmp_dir.path().join("TEST.DAT");
        let mut dat_file = match DATFile::create_unsafe(&tmp_path, DATType::Macro, 4, 8, 0) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Error creating temp file: {}", err)),
        };
        match dat_file.set_max_size(6) {
            Ok(_) => {
                assert_eq!(dat_file.max_size(), 6);
                // File size on disk should equal max size + MAX_SIZE_OFFSET.
                let meta = match dat_file.metadata() {
                    Ok(meta) => meta,
                    Err(err) => return Err(format!("Could not read back content: {}", err)),
                };
                assert_eq!(meta.len(), 6 + MAX_SIZE_OFFSET as u64);
                Ok(())
            }
            Err(err) => return Err(format!("Error setting content size: {}", err)),
        }
    }

    #[test]
    fn test_datfile_set_max_size_error_zero_size() -> Result<(), String> {
        let tmp_dir = match tempdir() {
            Ok(tmp_dir) => tmp_dir,
            Err(err) => return Err(format!("Error creating temp dir: {}", err)),
        };
        let tmp_path = tmp_dir.path().join("TEST.DAT");
        let mut dat_file = match DATFile::create_unsafe(&tmp_path, DATType::Macro, 4, 8, 0) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Error creating temp file: {}", err)),
        };
        match dat_file.set_max_size(0) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err {
                DATError::InvalidInput(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_datfile_set_max_size_error_under_content() -> Result<(), String> {
        let tmp_dir = match tempdir() {
            Ok(tmp_dir) => tmp_dir,
            Err(err) => return Err(format!("Error creating temp dir: {}", err)),
        };
        let tmp_path = tmp_dir.path().join("TEST.DAT");
        let mut dat_file = match DATFile::create_unsafe(&tmp_path, DATType::Unknown, 4, 8, 0) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Error creating temp file: {}", err)),
        };
        match dat_file.set_max_size(2) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err {
                DATError::Overflow(_) => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_datfile_read() -> Result<(), String> {
        let mut dat_file = match DATFile::open(TEST_PATH) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Open error: {}", err)),
        };
        let mut buf = [0u8; 1];
        match dat_file.read(&mut buf) {
            Ok(_) => Ok(assert_eq!(buf, TEST_CONTENTS[0..1])),
            Err(err) => Err(format!("Read error: {}", err)),
        }
    }

    #[test]
    fn test_datfile_read_with_mask() -> Result<(), String> {
        let mut dat_file = match DATFile::open(TEST_XOR_PATH) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Open error: {}", err)),
        };
        let mut buf = [0u8; 1];
        match dat_file.read(&mut buf) {
            Ok(_) => Ok(assert_eq!(buf, TEST_XOR_CONTENTS[0..1])),
            Err(err) => Err(format!("Read error: {}", err)),
        }
    }

    #[test]
    fn test_datfile_read_past_end() -> Result<(), String> {
        let mut dat_file = match DATFile::open(TEST_PATH) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Open error: {}", err)),
        };
        let mut buf = [1u8; 8];
        match dat_file.read(&mut buf) {
            Ok(_) => {
                assert_eq!(&buf[0..5], TEST_CONTENTS);
                // Bytes past content end should be untouched.
                assert_eq!(buf[5..], [1u8; 3]);
                Ok(())
            }
            Err(err) => Err(format!("Read error: {}", err)),
        }
    }

    #[test]
    fn test_datfile_seek_current() -> Result<(), String> {
        let mut dat_file = match DATFile::open(TEST_PATH) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Open error: {}", err)),
        };
        match dat_file.seek(SeekFrom::Current(1)) {
            // Seek should be 1 byte into content
            Ok(_) => Ok(assert_eq!(
                dat_file.raw_file.stream_position().unwrap(),
                HEADER_SIZE as u64 + 1
            )),
            Err(err) => Err(format!("Seek error: {}", err)),
        }
    }

    #[test]
    fn test_datfile_seek_start() -> Result<(), String> {
        let mut dat_file = match DATFile::open(TEST_PATH) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Open error: {}", err)),
        };
        match dat_file.seek(SeekFrom::Start(1)) {
            // Seek should be 1 byte into content
            Ok(_) => Ok(assert_eq!(
                dat_file.raw_file.stream_position().unwrap(),
                HEADER_SIZE as u64 + 1
            )),
            Err(err) => Err(format!("Seek error: {}", err)),
        }
    }

    #[test]
    fn test_datfile_seek_end() -> Result<(), String> {
        let mut dat_file = match DATFile::open(TEST_PATH) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Open error: {}", err)),
        };
        match dat_file.seek(SeekFrom::End(-1)) {
            // Seek should be 1 byte from content (end measured without including the terminating null byte)
            Ok(_) => Ok(assert_eq!(
                dat_file.raw_file.stream_position().unwrap(),
                HEADER_SIZE as u64 + dat_file.content_size() as u64 - 2
            )),
            Err(err) => Err(format!("Seek error: {}", err)),
        }
    }

    #[test]
    fn test_datfile_seek_current_error_negative() -> Result<(), String> {
        let mut dat_file = match DATFile::open(TEST_PATH) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Open error: {}", err)),
        };
        match dat_file.seek(SeekFrom::Current(-10)) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err.kind() {
                std::io::ErrorKind::InvalidInput => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }

    #[test]
    fn test_datfile_write() -> Result<(), String> {
        // Make a tempfile
        let tmp_dir = match tempdir() {
            Ok(tmp_dir) => tmp_dir,
            Err(err) => return Err(format!("Error creating temp dir: {}", err)),
        };
        let tmp_path = tmp_dir.path().join("TEST.DAT");
        match copy(TEST_PATH, &tmp_path) {
            Ok(_) => (),
            Err(err) => return Err(format!("Could not create temp file for testing: {}", err)),
        };
        // Open tempfile
        let mut opts = OpenOptions::new();
        opts.read(true).write(true);
        let mut dat_file = match DATFile::open_options(&tmp_path, &mut opts) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Error opening temp file: {}", err)),
        };
        // Write
        let new_content = b"Hi!";
        match dat_file.write(new_content) {
            Ok(_) => (),
            Err(err) => return Err(format!("Error writing content: {}", err)),
        };
        // Seek back to start
        match dat_file.seek(SeekFrom::Start(0)) {
            Ok(_) => (),
            Err(err) => return Err(format!("Error seeking in file: {}", err)),
        }
        // Check content
        match read_content(&tmp_path) {
            Ok(content_bytes) => Ok(assert_eq!(&content_bytes, b"Hi!p!")),
            Err(err) => Err(format!("Error reading file after write: {}", err)),
        }
    }

    #[test]
    fn test_datfile_write_extend_content_size() -> Result<(), String> {
        // Make a tempfile
        let tmp_dir = match tempdir() {
            Ok(tmp_dir) => tmp_dir,
            Err(err) => return Err(format!("Error creating temp dir: {}", err)),
        };
        let tmp_path = tmp_dir.path().join("TEST.DAT");
        match copy(TEST_EMPTY_PATH, &tmp_path) {
            Ok(_) => (),
            Err(err) => return Err(format!("Could not create temp file for testing: {}", err)),
        };
        // Open tempfile
        let mut opts = OpenOptions::new();
        opts.read(true).write(true);
        let mut dat_file = match DATFile::open_options(&tmp_path, &mut opts) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Error opening temp file: {}", err)),
        };
        // Write
        let new_content = b"Long!";
        match dat_file.write(new_content) {
            Ok(_) => (),
            Err(err) => return Err(format!("Error writing content: {}", err)),
        };
        // Seek back to start
        match dat_file.seek(SeekFrom::Start(0)) {
            Ok(_) => (),
            Err(err) => return Err(format!("Error seeking in file: {}", err)),
        }
        // Check content
        match read_content(&tmp_path) {
            Ok(content_bytes) => {
                assert_eq!(&content_bytes, new_content);
                assert_eq!(dat_file.content_size(), new_content.len() as u32 + 1);
                Ok(())
            }
            Err(err) => Err(format!("Error reading file after write: {}", err)),
        }
    }

    #[test]
    fn test_datfile_write_error_over_max_size() -> Result<(), String> {
        // Make a tempfile
        let tmp_dir = match tempdir() {
            Ok(tmp_dir) => tmp_dir,
            Err(err) => return Err(format!("Error creating temp dir: {}", err)),
        };
        let tmp_path = tmp_dir.path().join("TEST.DAT");
        match copy(TEST_EMPTY_PATH, &tmp_path) {
            Ok(_) => (),
            Err(err) => return Err(format!("Could not create temp file for testing: {}", err)),
        };
        // Open tempfile
        let mut opts = OpenOptions::new();
        opts.read(true).write(true);
        let mut dat_file = match DATFile::open_options(&tmp_path, &mut opts) {
            Ok(dat_file) => dat_file,
            Err(err) => return Err(format!("Error opening temp file: {}", err)),
        };
        // Write
        let new_content = b"Looooooooooooooooooong!";
        match dat_file.write(new_content) {
            Ok(_) => Err("No error returned.".to_owned()),
            Err(err) => match err.kind() {
                std::io::ErrorKind::InvalidInput => Ok(()),
                _ => Err(format!("Incorrect error: {}", err)),
            },
        }
    }
}
