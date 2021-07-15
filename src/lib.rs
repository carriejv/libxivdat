// Copyright 2021 Carrie J Vrtis
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A Rust library for working with Final Fantasy XIV .DAT files.
//! These files store client-side game config including macros, hotkeys, and ui settings.
//!
//! Libxivdat provides low-level file i/o via [`DATFile`](crate::dat_file::DATFile),
//! a [`std::fs::File`]-like interface that automatically manages the header, footer, and content
//! masking of DAT files.
//!
//! Each DAT file contains unique data structures. Higher-level support for specific file types is
//! implemented on a type-by-type basis as optional features. See the chart below
//!  for more information and feature names.
//!
//! # DAT Data Structures
//!
//! Internally, some DAT file content blocks use a variable-length data structure referred to as a [`section`](crate::section)
//! in this library. A section consists of a single UTF-8 char type tag, u16le size, and a null-terminated
//! UTF-8 string. A single resource (ie, a macro) is then comprised of a repeating pattern of sections.
//! Other DAT files use fixed-size resource blocks, with each resource immediately following the last.
//! These are referred to as "Block DATs" below.
//!
//! Some DAT files contain unique binary data that does not follow the "standard" DAT format. Others contain
//! UTF-8 plaintext and are not binary files at all. Support for these files is not currently planned.
//!
//! ## DAT Support Table
//!
//! | Symbol | Description     |
//! |--------|-----------------|
//! |   ✅   | Full support    |
//! |   🌀   | Partial support |
//! |   ❌   | No support      |
//!
//! | File               | Contains                         | Type       | DATFile Read/Write | High Level Module |
//! |--------------------|----------------------------------|------------|--------------------|-------------------|
//! | ACQ.DAT            | Recent /tell history             | Section    |         ✅         |   🌀 - `section`  |
//! | ADDON.DAT          | ?                                | Unique     |         ❌         |         ❌        |
//! | COMMON.DAT         | Character configuration          | Plaintext  |         ❌         |         ❌        |
//! | CONTROL0.DAT       | Gamepad control config           | Plaintext  |         ❌         |         ❌        |
//! | CONTROL1.DAT       | Keyboard/mouse control config    | Plaintext  |         ❌         |         ❌        |
//! | FFXIV_CHARA_XX.DAT | Character appearance presets     | Unique     |         ❌         |         ❌        |
//! | GEARSET.DAT        | Gearsets                         | Block      |         ✅         |         ❌        |
//! | GS.DAT             | Gold Saucer config (Triad decks) | Block      |         ✅         |         ❌        |
//! | HOTBAR.DAT         | Hotbar layouts                   | Block      |         ✅         |         ❌        |
//! | ITEMFDR.DAT        | "Search for item" indexing?      | Block      |         ✅         |         ❌        |
//! | ITEMODR.DAT        | Item order in bags               | Block      |         ✅         |         ❌        |
//! | KEYBIND.DAT        | Keybinds                         | Section    |         ✅         |   🌀 - `section`  |
//! | LOGFLTR.DAT        | Chat log filters?                | Block      |         ✅         |         ❌        |
//! | MACRO.DAT          | Character-specific macros        | Section    |         ✅         |    ✅ - `macro`   |
//! | MACROSYS.DAT       | System-wide macros               | Section    |         ✅         |    ✅ - `macro`   |
//! | UISAVE.DAT         | UI config                        | Block      |         ✅         |         ❌        |
//!
//! # Examples:
//!
//! ## Reading a file:
//!
//! ```rust
//! use libxivdat::dat_file::read_content;
//! # let path_to_dat_file = "./resources/TEST.DAT";
//! let data_vec = read_content(&path_to_dat_file).unwrap();
//! ```
//!
//! ## Writing to an existing file:
//! DAT files contain metadata in the header that pertains to how data should be written.
//! Because of this, creating a new DAT file and writing contents are separate steps.
//!
//! ```rust
//! use libxivdat::dat_file::write_content;
//! # use libxivdat::dat_file::DATFile;
//! # use libxivdat::dat_type::DATType;
//! # extern crate tempfile;
//! # use tempfile::tempdir;
//! # let temp_dir = tempdir().unwrap();
//! # let path_to_dat_file = temp_dir.path().join("TEST.DAT");
//! # DATFile::create(&path_to_dat_file, DATType::Macro).unwrap();
//! let data_vec = write_content(&path_to_dat_file, b"This is some data.").unwrap();
//! ```
//!
//! ## Creating a new file:
//!
//! ```rust
//! use libxivdat::dat_file::DATFile;
//! use libxivdat::dat_type::DATType;
//! # extern crate tempfile;
//! # use tempfile::tempdir;
//! # let temp_dir = tempdir().unwrap();
//! # let path_to_dat_file = temp_dir.path().join("TEST.DAT");
//! DATFile::create_with_content(&path_to_dat_file, DATType::Macro, b"This is some data.").unwrap();
//! ```
//!
//! ## File-like access:
//!
//! ```rust
//! use libxivdat::dat_file::read_content;
//! use libxivdat::dat_file::DATFile;
//! use libxivdat::dat_type::DATType;
//! use std::io::{Read,Seek,SeekFrom,Write};
//! # extern crate tempfile;
//! # use tempfile::tempdir;
//! # let temp_dir = tempdir().unwrap();
//! # let path_to_dat_file = temp_dir.path().join("TEST.DAT");
//! # DATFile::create(&path_to_dat_file, DATType::Macro).unwrap();
//!
//! let mut dat_file = DATFile::open(&path_to_dat_file).unwrap();
//!
//! let mut first_256_bytes = [0u8; 256];
//! dat_file.read(&mut first_256_bytes).unwrap();
//! ```

/// Contains the [`DATError`](crate::dat_error::DATError) wrapper error. This error type is used
/// for all functions that do not implement a `std::io` trait.
pub mod dat_error;
/// Contains a generic, low-level tool set for working with any standard binary DAT files.
/// This provides the convenience functions [`read_content()`](crate::dat_file::read_content)
/// and [`write_content()`](crate::dat_file::write_content) as well as the [`std::fs::File`]-like
/// [`DATFile`](crate::dat_file::DATFile) interface.
pub mod dat_file;
/// Contains the enum of all supported file types, [`DATType`](crate::dat_type::DATType) and
/// functions for accessing default header and mask values specific to each type.
pub mod dat_type;
/// Contains general-purpose traits and functions applicable to all high-level, file-type-specific
/// modules such as [`xiv_macro`].
///
/// Enabled by feature `high-level`, which is implied by any file type feature.
#[cfg(feature = "high-level")]
pub mod high_level;
/// Contains a generic tool set for working with any section-based binary DAT files.
/// This module contains two equivalent implementations: [`Section`](crate::section::Section),
/// [`read_section()`](crate::section::read_section), and [`read_section_content()`](crate::section::read_section_content)
/// for working with files on disk and [`SectionData`](`crate::section::SectionData),
/// [`as_section()`](crate::section::as_section`), and [`as_section_vec()`](crate::section::as_section_vec)
/// for working with pre-allocated byte arrays.
///
/// Because sections are variable-length data structures, no functions for writing sections in-place are
/// provided. The recommended approach to writing section-based files is to read the entire file, then
/// write an entirely new content block with [`write_content()`](crate::dat_file::write_content).
pub mod section;
/// Contains the high-level toolkit for working with macro files, `MACRO.DAT` and `MACROSYS.DAT`.
/// This module contains two equivalent implementations: [`Macro`](crate::xiv_macro::Macro),
/// [`read_macro()`](crate::xiv_macro::read_macro), and [`read_macro_content()`](crate::xiv_macro::read_macro_content)
/// for working with files on disk and [`MacroData`](`crate::xiv_macro::MacroData),
/// [`as_macro()`](crate::xiv_macro::as_macro`), and [`as_macro_vec()`](crate::xiv_macro::as_macro_vec)
/// for working with pre-allocated byte arrays and [`SectionData](crate::section::SectionData).
///
/// Enabled by feature `macro`.
///
/// # Examples
///
/// ## Reading a macro file
/// ```rust
/// use libxivdat::xiv_macro::read_macro_content;
/// use libxivdat::xiv_macro::icon::MacroIcon;
///
/// let macro_contents = read_macro_content("./resources/TEST_MACRO.DAT").unwrap();
///
/// assert_eq!(macro_contents[0].title, "0");
/// assert_eq!(macro_contents[0].lines[0], "DefaultIcon");
/// assert_eq!(macro_contents[0].get_icon().unwrap(), MacroIcon::DefaultIcon);
///
/// assert_eq!(macro_contents[1].title, "1");
/// assert_eq!(macro_contents[1].lines[0], "DPS1");
/// assert_eq!(macro_contents[1].get_icon().unwrap(), MacroIcon::DPS1);
/// ```
///
/// ## Writing a macro file
/// Macro files use variable-length [`Sections`](crate::section::Section) to store data on disk,
/// so it is not possible to easily overwrite a single macro in-place. The recommended approach to modifying
/// macros is to read and write the entire file as a block.
/// ```rust
/// use libxivdat::dat_file::write_content;
/// use libxivdat::xiv_macro::{read_macro_content, to_writeable_bytes, Macro};
/// use libxivdat::xiv_macro::icon::MacroIcon;
/// # use libxivdat::dat_file::DATFile;
/// # use libxivdat::dat_type::DATType;
///
/// # extern crate tempfile;
/// # use tempfile::tempdir;
/// # let temp_dir = tempdir().unwrap();
/// # let out_path = temp_dir.path().join("TEST.DAT");
/// # DATFile::create(&out_path, DATType::Macro).unwrap();
///
/// let mut macro_vec = read_macro_content("./resources/TEST_MACRO.DAT").unwrap();
/// // Replace macro #0 with a new macro. Macro::new will enforce the game's specs for macros.
/// macro_vec[0] = Macro::new(
///     String::from("libxivdat was here"),
///     vec![String::from("/sh I <3 libxivdat!!!")],
///     MacroIcon::SymbolExclamation
/// ).unwrap();
///
/// // Write it back out to a file. to_writeable_bytes will validate every macro in the vector.
/// let out_bytes = to_writeable_bytes(&macro_vec).unwrap();
/// write_content(&out_path, &out_bytes);
/// ```
#[cfg(feature = "macro")]
pub mod xiv_macro {
    pub use crate::high_level_modules::r#macro::*;
}
/// High-level, file-type-specific submodules container.
mod high_level_modules;
