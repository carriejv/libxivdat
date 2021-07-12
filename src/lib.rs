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
//! Libxivdat currently provides only low-level access via [`DATFile`](crate::dat_file::DATFile),
//! a [`std::fs::File`]-like interface that automatically manages the header, footer, and content
//! masking of DAT files.
//!
//! Each DAT file contains unique data structures. Higher-level modules for individual DAT files
//! are planned for reading/writing entire resources (ie, macros or gearsets) but are not yet
//! implemented.
//!
//! # DAT Data Structures
//!
//! Internally, some DAT file content blocks use a variable-length data structure referred to as a `Section`
//! in this library. A Section consists of a single UTF-8 char type tag, u16le size, and a null-terminated
//! UTF-8 string. A single resource (ie, a macro) is then comprised of a repeating pattern of Sections.
//! Other DAT files use fixed-size resource blocks, with each resource immediately following the last.
//! These are referred to as "Block DATs" below.
//!
//! Some DAT files contain unique binary data that does not follow the "standard" DAT format. Others contain
//! UTF-8 plaintext and are not binary files at all. Support for these files is not currently planned.
//!
//! ## DAT Support Table
//!
//! | File               | Contains                         | Data Type  | DATFile Read/Write | High Level Module |
//! |--------------------|----------------------------------|------------|--------------------|-------------------|
//! | ACQ.DAT            | Recent /tell history             | Section    |         ✅         |         ❌        |
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
//! | KEYBIND.DAT        | Keybinds                         | Section    |         ✅         |         ❌        |
//! | LOGFLTR.DAT        | Chat log filters?                | Block      |         ✅         |         ❌        |
//! | MACRO.DAT          | Character-specific macros        | Section    |         ✅         |         ❌        |
//! | MACROSYS.DAT       | System-wide macros               | Section    |         ✅         |         ❌        |
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
/// TODO
pub mod section;
