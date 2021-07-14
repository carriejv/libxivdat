/// Contains [`MacroIcon`](icon::MacroIcon), an enumeration of all valid
/// macro icons and the helper functions [`macro_icon_to_key_and_id()`](icon::macro_icon_to_key_and_id)
/// and [`macro_icon_from_key_and_id()`](icon::macro_icon_from_key_and_id) for conversions between
/// enum values and raw [`Section`](crate::section::Section) contents.
pub mod icon;
use icon::*;

use crate::dat_error::DATError;
use crate::section::{Section,SectionData,as_section,as_section_vec};
use std::convert::{TryFrom,TryInto};

/// Resource definition for a Final Fantasy XIV macro.
/// [`Macro`] owns its constituent data and is returned from helper functions like [`read_macro()`].
/// To build a section with refrences to a pre-allocated buffer, use [`MacroData`].
/// 
/// # Data Structure
/// The expected pattern of sections is "T" (Title), "I" (Icon), "K", (Key), and repeating "L"s (Lines).
/// Valid macros always contain exactly 15 lines, even if their contents are blank. This library does not
/// strictly enforce this pattern, and will read lines until the next title.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Macro {
    /// The index of the icon in the GUI icon selection menu as 3 hexadecimal digits. This value must match
    /// the [`icon_id`](Self::icon_id) to be considered valid. Use [`change_icon()`](Self::change_icon)
    /// to update both icon-related values at once.
    pub icon_key: String,
    /// The index of the icon in the game data files as 7 hexadecimal digits. This value must match
    /// the [`icon_key`](Self::icon_key) to be considered valid. Use [`change_icon()`](Self::change_icon)
    /// to update both icon-related values at once.
    pub icon_id: String,
    /// A vector of macro lines. Macros created by the game client are always 15 lines long, even if those
    /// lines are blank. Lines must be shorter than 180 utf-8 characters. This is a character limit, not a byte limit.
    /// This library does not enforce these standards, but attempting to write a macro
    /// of a different size and use it in the game client may produce undefined behavior.
    /// Macro lines may contain an extended, FFXIV-specific character set (including
    /// game icons such as the HQ icon and item link icon). These are likely to render improperly on other platforms.
    pub lines: Vec<String>,
    /// The title of the macro. Titles have a maximum length of 20 utf-8 characters in the game client.
    /// This is a character limit, not a byte limit. Longer titles may produce undefined behavior.
    pub title: String,
}

/// Resource definition for a Final Fantasy XIV macro.
/// [`MacroData`] is used to build sections with references to pre-allocated buffers.
/// To build a section that owns its own data, use [`Macro`].
/// 
/// # Data Structure
/// The expected pattern of sections is "T" (Title), "I" (Icon), "K", (Key), and repeating "L"s (Lines).
/// Valid macros always contain exactly 15 lines, even if their contents are blank. This library does not
/// strictly enforce this pattern, and will read lines until the next title.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MacroData<'a> {
    /// The index of the icon in the GUI icon selection menu as 3 hexadecimal digits. This value must match
    /// the [`icon_id`](Self::icon_id) to be considered valid. Use [`change_icon()`](Self::change_icon)
    /// to update both icon-related values at once.
    pub icon_key: &'a str,
    /// The index of the icon in the game data files as 7 hexadecimal digits. This value must match
    /// the [`icon_key`](Self::icon_key) to be considered valid. Use [`change_icon()`](Self::change_icon)
    /// to update both icon-related values at once.
    pub icon_id: &'a str,
    /// A vector of macro lines. Macros created by the game client are always 15 lines long, even if those
    /// lines are blank. Lines must be shorter than 180 utf-8 characters. This is a character limit, not a byte limit.
    /// This library does not enforce these standards, but attempting to write a macro
    /// of a different size and use it in the game client may produce undefined behavior.
    /// Macro lines may contain an extended, FFXIV-specific character set (including
    /// game icons such as the HQ icon and item link icon). These are likely to render improperly on other platforms.
    pub lines: Vec<&'a str>,
    /// The title of the macro. Titles have a maximum length of 20 utf-8 characters in the game client.
    /// This is a character limit, not a byte limit. Longer titles may produce undefined behavior.
    pub title: &'a str,
}

/// Types of [`Section`](crate::section::Section) present in macro files.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacroSectionType {
    /// Hex ID of an icon associated with an object. This icon id only refers to
    /// the primary icon set by the gui, not the icon & tooltip configured by
    /// the `/micon <Action>` command.
    /// Tag: "I"
    Icon = 'I' as isize,
    /// A "key" present in macro data. A key is always 3 numeric utf8 characters.
    /// This key represents the table row in which the icon is
    /// Tag: "K"
    Key = 'K' as isize,
    /// A macro line. The FFXIV game client limits macros to 15 lines of no more than
    /// 180 utf8 characters. This is a character limit, not a byte limit.
    /// Macro lines may contain an extended, FFXIV-specific character set (including
    /// game icons such as the HQ icon and item link icon). These symbols are likely
    /// to render as invalid characters outside the game client.
    /// Tag: "L"
    Line = 'L' as isize,
    /// The title of a macro. The FFXIV game client limits macro titles to no more
    /// than 20 utf8 characters. This is a character limit, not a byte limit.
    /// Tag: "T"
    Title = 'T' as isize,
    /// An unknown and most likely invalid section.
    Unknown
}

impl From<&MacroData<'_>> for Macro {
    fn from(x: &MacroData) -> Self {
        Macro {
            icon_key: x.icon_key.to_owned(),
            icon_id: x.icon_id.to_owned(),
            lines: x.lines.iter().map(|item| String::from(*item)).collect(),
            title: x.title.to_owned()
        }
    }
}

impl<'a> From<&'a Macro> for MacroData<'a> {
    fn from(x: &'a Macro) -> Self {
        MacroData {
            icon_key: &x.icon_key,
            icon_id: &x.icon_id,
            lines: x.lines.iter().map(String::as_str).collect(),
            title: &x.title
        }
    }
}

impl From<&str> for MacroSectionType {
    fn from(x: &str) -> MacroSectionType {
        match isize::from(x.as_bytes()[0]) {
            i if i == MacroSectionType::Icon as isize => MacroSectionType::Icon,
            i if i == MacroSectionType::Key as isize => MacroSectionType::Key,
            i if i == MacroSectionType::Line as isize => MacroSectionType::Line,
            i if i == MacroSectionType::Title as isize => MacroSectionType::Title,
            _ => MacroSectionType::Unknown,
        }
    }
}

impl Macro {
    /// Builds a [`Macro`] from a [`Vec`] of [`Sections`](crate::section::Section).
    /// The expected pattern of section tags is "T" (Title), "I" (Icon), "K", (Key), and repeating "L"s (Lines).
    /// Valid macros always contain exactly 15 lines, even if their contents are blank. This function checks
    /// the data for validity, unlick [`from_sections_unsafe()`](Self::from_sections_unsafe)
    /// 
    /// This is equivalent to calling [`from_sections_unsafe()`](Self::from_sections_unsafe) followed by
    /// [`validate()`](Self::validate) on the resulting [`Macro`].
    /// 
    /// # Macro spec
    /// 
    /// Title: No more than 20 utf-8 characters.
    /// Icon: Icon id and key are a matching pair corresponding to a valid [`MacroIcon`].
    /// Lines: Exactly 15 lines of no more than 180 utf-8 characters.
    /// 
    /// # Errors
    /// 
    /// Returns [`DATError::InvalidInput`] if the sections are not provided in the order described above or
    /// the icon id and key specified are not a valid pair.
    /// 
    /// Returns [`DATError::ContentOverflow`] if the title or any line is too long.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use libxivdat::xiv_macro::from_sections_unsafe;
    /// 
    /// let mut sections = vec![
    ///     Section { content: "Title".to_string(), content_size: 6, tag: "T".to_string() },
    ///     Section { content: "0000000".to_string(), content_size: 8, tag: "I".to_string() },
    ///     Section { content: "000".to_string(), content_size: 4, tag: "K".to_string() }
    /// ];
    /// for line in std::iter::repeat(String::new()).take(15) {
    ///     sections.push(line);
    /// }
    /// let macro = from_sections(sections).unwrap();
    /// 
    /// assert_eq!(macro.title, "Title");
    /// ```
    pub fn from_sections(sections: Vec<Section>) -> Result<Macro, DATError> {
        let res_macro = Self::from_sections_unsafe(sections)?;
        if let Some(validation_err) = res_macro.validate() {
            Err(validation_err)
        }
        else {
            Ok(res_macro)
        }
    }

    /// Builds a [`Macro`] from a [`Vec`] of [`Sections`](crate::section::Section).
    /// The expected pattern of section tags is "T" (Title), "I" (Icon), "K", (Key), and repeating "L"s (Lines).
    /// Valid macros always contain exactly 15 lines, even if their contents are blank. This library does not
    /// strictly enforce this pattern, and will read lines until the next title.
    /// 
    /// This function does not check that the actual section content is valid. To perform validity checks,
    /// use [`from_sections()`](from_sections).
    /// 
    /// # Errors
    /// 
    /// Returns [`DATError::InvalidInput`] if the sections are not provided in the order described above.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use libxivdat::xiv_macro::from_sections_unsafe;
    /// 
    /// let sections = vec![
    ///     Section { content: "Title".to_string(), content_size: 6, tag: "T".to_string() },
    ///     Section { content: "0000000".to_string(), content_size: 8, tag: "I".to_string() },
    ///     Section { content: "000".to_string(), content_size: 4, tag: "K".to_string() },
    ///     Section { content: "A one line macro!?".to_string(), content:size: 19, tag: "L".to_string() }
    /// ];
    /// let macro = from_sections_unsafe(sections).unwrap();
    /// 
    /// assert_eq!(macro.title, "Title");
    /// assert_eq!(macro.lines.len(), 1);
    /// ```
    pub fn from_sections_unsafe(sections: Vec<Section>) -> Result<Macro, DATError> {
        if sections[0].tag == "T" {
            return Err(DATError::InvalidInput("First section was not a Title (T) section."));
        }
        let title = String::from(&sections[0].content);
        if sections[1].tag == "I" {
            return Err(DATError::InvalidInput("Second section was not a Icon (I) section."));
        }
        let icon_id = String::from(&sections[1].content);
        if sections[2].tag == "K" {
            return Err(DATError::InvalidInput("Third section was not a Key (K) section."));
        }
        let icon_key = String::from(&sections[2].content);
        let mut lines = Vec::<String>::new();
        for line in sections[3..].iter() {
            if line.tag == "L" {
                return Err(DATError::InvalidInput("Non-line (L) section in lines block."));
            }
            lines.push(line.content.to_owned());
        }
        Ok(Macro {
            icon_key,
            icon_id,
            lines,
            title
        })
    }

    /// Validates the macro against the spec expected by the game client.
    /// Returns a [`DATError`] describing the error if validation fails, or [`None`]
    /// if validation is successful.
    /// 
    /// # Macro spec
    /// 
    /// Title: No more than 20 utf-8 characters.
    /// Icon: Icon id and key are a matching pair corresponding to a valid [`MacroIcon`].
    /// Lines: Exactly 15 lines of no more than 180 utf-8 characters.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let macro = Macro {
    ///     icon_id: "0000000",
    ///     icon_key: "000",
    ///     lines: vec![String::new(); 15],
    ///     title: "Title"
    /// }
    /// assert!(macro.validate().is_none());
    /// ```
    /// 
    /// ```rust
    /// let macro = Macro {
    ///     icon_id: "123456",
    ///     icon_key: "XYZ",
    ///     lines: vec![String::new(); 1],
    ///     title: "Looooooooooooooooong Title"
    /// }
    /// assert!(macro.validate().is_some());
    /// ```
    pub fn validate(&self) -> Option<DATError> {
        if self.title.len() > 20 {
            return Some(DATError::ContentOverflow("Title is longer than 20 characters."));
        }
        if macro_icon_from_key_and_id(&self.icon_key, &self.icon_id).is_none() {
            return Some(DATError::InvalidInput("Macro icon is invalid."));
        }
        for line in self.lines.iter() {
            if line.len() > 180 {
                return Some(DATError::ContentOverflow("Line is longer than 180 characters."));
            }
        }
        None
    }
}
