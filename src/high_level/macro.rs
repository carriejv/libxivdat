/// Contains [`MacroIcon`](icon::MacroIcon), an enumeration of all valid
/// macro icons and the helper functions [`macro_icon_to_key_and_id()`](icon::macro_icon_to_key_and_id)
/// and [`macro_icon_from_key_and_id()`](icon::macro_icon_from_key_and_id) for conversions between
/// enum values and raw [`Section`](crate::section::Section) contents.
pub mod icon;
use icon::*;

use crate::dat_error::DATError;
use crate::dat_file::{DATFile,check_type,write_content};
use crate::dat_type::DATType;
use crate::section::{Section,SectionData,as_section,as_section_vec,read_section,read_section_content};
use std::path::Path;

/// The [`Section`](crate::section::Section) tag for macro titles.
pub const SECTION_TAG_TITLE: &'static str = "T";

/// The [`Section`](crate::section::Section) tag for macro icon ids.
pub const SECTION_TAG_ICON: &'static str = "I";

/// The [`Section`](crate::section::Section) tag for macro icon keys.
pub const SECTION_TAG_KEY: &'static str = "K";

/// The [`Section`](crate::section::Section) tag for macro icon lines.
pub const SECTION_TAG_LINE: &'static str = "L";

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

impl From<&MacroData<'_>> for Macro {
    fn from(x: &MacroData) -> Self {
        Macro {
            icon_key: x.icon_key.to_owned(),
            icon_id: x.icon_id.to_owned(),
            lines: x.lines.iter().map(|item| String::from(*item)).collect(),
            title: x.title.to_owned(),
        }
    }
}

impl<'a> From<&'a Macro> for MacroData<'a> {
    fn from(x: &'a Macro) -> Self {
        MacroData {
            icon_key: &x.icon_key,
            icon_id: &x.icon_id,
            lines: x.lines.iter().map(String::as_str).collect(),
            title: &x.title,
        }
    }
}

impl Macro {
    /// Returns a byte vector representing the [`Macro`].
    ///
    /// # Errors
    ///
    /// Returns a [`DATError::ContentOverflow`] if the content of a section would exceed
    /// the maximum allowable length. ([`u16::MAX`]` - 1`)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::xiv_macro::Macro;
    /// use libxivdat::xiv_macro::icon::MacroIcon;
    ///
    /// let a_macro = Macro::new(
    ///     "Title".to_string(),
    ///     vec!["Circle".to_string()],
    ///     MacroIcon::SymbolCircle
    /// ).unwrap();
    ///
    /// let bytes = a_macro.as_bytes();
    /// assert!(bytes.is_ok());
    /// ```
    pub fn as_bytes(&self) -> Result<Vec<u8>, DATError> {
        let sections = self.as_sections()?;
        let mut byte_vec = Vec::<u8>::new();
        for section in sections.into_iter() {
            let mut sec_bytes = Vec::<u8>::from(section);
            byte_vec.append(&mut sec_bytes);
        }
        Ok(byte_vec)
    }

    /// Returns a [`Vec`] of [`Sections`](crate::section::Section) representing the
    /// [`Macro`].
    ///
    /// # Errors
    ///
    /// Returns a [`DATError::ContentOverflow`] if the content of a section would exceed
    /// the maximum allowable length. ([`u16::MAX`]` - 1`)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::xiv_macro::Macro;
    /// use libxivdat::xiv_macro::icon::MacroIcon;
    ///
    /// let a_macro = Macro::new(
    ///     "Title".to_string(),
    ///     vec!["Circle".to_string()],
    ///     MacroIcon::SymbolCircle
    /// ).unwrap();
    ///
    /// let sections = a_macro.as_sections().unwrap();
    ///
    /// assert_eq!(sections[0].content, "Title");
    /// ```
    pub fn as_sections(&self) -> Result<Vec<Section>, DATError> {
        let mut sec_vec = vec![
            Section::new(SECTION_TAG_TITLE.to_owned(), String::from(&self.title))?,
            Section::new(SECTION_TAG_ICON.to_owned(), String::from(&self.icon_id))?,
            Section::new(SECTION_TAG_KEY.to_owned(), String::from(&self.icon_key))?,
        ];
        for line in self.lines.iter() {
            sec_vec.push(Section::new(SECTION_TAG_LINE.to_owned(), String::from(line))?);
        }
        Ok(sec_vec)
    }

    /// Changes the [`icon_key`](Self.icon_key) and [`icon_id`](Self.icon_id) to a valid pair based on an input
    /// [`MacroIcon`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::xiv_macro::Macro;
    /// use libxivdat::xiv_macro::icon::MacroIcon;
    ///
    /// let mut a_macro = Macro::new(
    ///     "Title".to_string(),
    ///     Vec::<String>::new(),
    ///     MacroIcon::NoIcon
    /// ).unwrap();
    ///
    /// assert_eq!(a_macro.icon_id, "0000000");
    /// assert_eq!(a_macro.icon_key, "000");
    ///
    /// a_macro.change_icon(MacroIcon::SymbolArrowUp);
    /// assert_eq!(a_macro.icon_id, "00102FF");
    /// assert_eq!(a_macro.icon_key, "037");
    /// ```
    pub fn change_icon(&mut self, icon: MacroIcon) -> () {
        let (key, id) = macro_icon_to_key_and_id(&icon);
        self.icon_key = key.to_owned();
        self.icon_id = id.to_owned();
    }

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
    /// Returns [`DATError::ContentOverflow`] if the title or any line is too long, or if there are too many lines.
    ///
    /// Returns [`DATError::ContentUnderflow`] if there are too few lines.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::section::Section;
    /// use libxivdat::xiv_macro::Macro;
    ///
    /// let mut sections = vec![
    ///     Section { content: "Title".to_string(), content_size: 6, tag: "T".to_string() },
    ///     Section { content: "0000000".to_string(), content_size: 8, tag: "I".to_string() },
    ///     Section { content: "000".to_string(), content_size: 4, tag: "K".to_string() }
    /// ];
    /// for line in std::iter::repeat(String::new()).take(15) {
    ///     sections.push(Section { content: line, content_size: 1, tag: "L".to_string() });
    /// }
    /// let result_macro = Macro::from_sections(sections).unwrap();
    ///
    /// assert_eq!(result_macro.title, "Title");
    /// ```
    pub fn from_sections(sections: Vec<Section>) -> Result<Macro, DATError> {
        let res_macro = Self::from_sections_unsafe(sections)?;
        if let Some(validation_err) = res_macro.validate() {
            Err(validation_err)
        } else {
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
    /// Returns [`DATError::InvalidInput`] if the sections are not provided in the order described above
    /// or any sections are missing.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::section::Section;
    /// use libxivdat::xiv_macro::Macro;
    ///
    /// let sections = vec![
    ///     Section { content: "Title".to_string(), content_size: 6, tag: "T".to_string() },
    ///     Section { content: "0000000".to_string(), content_size: 8, tag: "I".to_string() },
    ///     Section { content: "000".to_string(), content_size: 4, tag: "K".to_string() },
    ///     Section { content: "A one line macro!?".to_string(), content_size: 19, tag: "L".to_string() }
    /// ];
    /// let result_macro = Macro::from_sections_unsafe(sections).unwrap();
    ///
    /// assert_eq!(result_macro.title, "Title");
    /// assert_eq!(result_macro.lines.len(), 1);
    /// ```
    pub fn from_sections_unsafe(sections: Vec<Section>) -> Result<Macro, DATError> {
        if sections.len() < 4 {
            return Err(DATError::InvalidInput("Macros require a minimum of 4 sections."));
        }
        if sections[0].tag != SECTION_TAG_TITLE {
            return Err(DATError::InvalidInput("First section was not a Title (T) section."));
        }
        let title = String::from(&sections[0].content);
        if sections[1].tag != SECTION_TAG_ICON {
            return Err(DATError::InvalidInput("Second section was not a Icon (I) section."));
        }
        let icon_id = String::from(&sections[1].content);
        if sections[2].tag != SECTION_TAG_KEY {
            return Err(DATError::InvalidInput("Third section was not a Key (K) section."));
        }
        let icon_key = String::from(&sections[2].content);
        let mut lines = Vec::<String>::new();
        for line in sections[3..].iter() {
            if line.tag != SECTION_TAG_LINE {
                return Err(DATError::InvalidInput("Non-line (L) section in lines block."));
            }
            lines.push(line.content.to_owned());
        }
        Ok(Macro {
            icon_key,
            icon_id,
            lines,
            title,
        })
    }

    /// Gets the [`MacroIcon`] correpsonding to the current [`icon_key`](Self.icon_key) and [`icon_id`](Self.icon_id).
    /// Returns [`None`] if the id and key do not correspond to a known valid icon.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::xiv_macro::Macro;
    /// use libxivdat::xiv_macro::icon::MacroIcon;
    ///
    /// let a_macro = Macro::new(
    ///     "Title".to_string(),
    ///     Vec::<String>::new(),
    ///     MacroIcon::SymbolArrowUp
    /// ).unwrap();
    /// assert_eq!(a_macro.get_icon().unwrap(), MacroIcon::SymbolArrowUp);
    /// ```
    pub fn get_icon(&self) -> Option<MacroIcon> {
        macro_icon_from_key_and_id(&self.icon_key, &self.icon_id)
    }

    /// Builds a new [`Macro`] with a given title, [`MacroIcon`], and content.
    /// This ensures that the macro meets the spec described below, which is used
    /// by the game client. If the provided line count is less than 15, the count
    /// will be padded with blank lines.
    ///
    /// To create an unvalidated [`Macro`], you can directly
    /// instantiate a struct literal.
    ///
    /// # Macro spec
    ///
    /// Title: No more than 20 utf-8 characters.
    /// Icon: Icon id and key are a matching pair corresponding to a valid [`MacroIcon`].
    /// Lines: Exactly 15 lines of no more than 180 utf-8 characters.
    ///
    /// # Errors
    ///
    /// Returns [`DATError::ContentOverflow`] if the title or content are too long, or if there
    /// are too many lines.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::xiv_macro::Macro;
    /// use libxivdat::xiv_macro::icon::MacroIcon;
    ///
    /// let a_macro = Macro::new(
    ///     "Title".to_string(),
    ///     vec!["Circle".to_string()],
    ///     MacroIcon::SymbolCircle
    /// ).unwrap();
    ///
    /// assert_eq!(a_macro.title, "Title");
    /// assert_eq!(a_macro.lines[0], "Circle");
    /// assert_eq!(a_macro.get_icon().unwrap(), MacroIcon::SymbolCircle);
    /// ```
    pub fn new(title: String, lines: Vec<String>, icon: MacroIcon) -> Result<Macro, DATError> {
        let (icon_key, icon_id) = macro_icon_to_key_and_id(&icon);
        let mut padded_lines = lines.clone();
        if lines.len() < 15 {
            for line in std::iter::repeat(String::new()).take(15 - lines.len()) {
                padded_lines.push(line);
            }
        }
        let res_macro = Macro {
            icon_id: icon_id.to_owned(),
            icon_key: icon_key.to_owned(),
            lines: padded_lines,
            title,
        };
        match res_macro.validate() {
            Some(err) => Err(err),
            None => Ok(res_macro),
        }
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
    /// use libxivdat::xiv_macro::Macro;
    ///
    /// let a_macro = Macro {
    ///     icon_id: "0000000".to_string(),
    ///     icon_key: "000".to_string(),
    ///     lines: vec![String::new(); 15],
    ///     title: "Title".to_string()
    /// };
    /// assert!(a_macro.validate().is_none());
    /// ```
    ///
    /// ```rust
    /// use libxivdat::xiv_macro::Macro;
    ///
    /// let a_macro = Macro {
    ///     icon_id: "123456".to_string(),
    ///     icon_key: "XYZ".to_string(),
    ///     lines: vec![String::new(); 1],
    ///     title: "Looooooooooooooooong Title".to_string()
    /// };
    /// assert!(a_macro.validate().is_some());
    /// ```
    pub fn validate(&self) -> Option<DATError> {
        if self.title.len() > 20 {
            return Some(DATError::ContentOverflow("Title is longer than 20 characters."));
        }
        if macro_icon_from_key_and_id(&self.icon_key, &self.icon_id).is_none() {
            return Some(DATError::InvalidInput("Macro icon is invalid."));
        }
        if self.lines.len() < 15 {
            return Some(DATError::ContentUnderflow("Macro has fewer than 15 lines."));
        }
        if self.lines.len() > 15 {
            return Some(DATError::ContentOverflow("Macro has more than 15 lines."));
        }
        for line in self.lines.iter() {
            if line.len() > 180 {
                return Some(DATError::ContentOverflow("Line is longer than 180 characters."));
            }
        }
        None
    }
}

impl<'a> MacroData<'a> {
    /// Returns a byte vector representing the [`MacroData`].
    ///
    /// # Errors
    ///
    /// Returns a [`DATError::ContentOverflow`] if the content of a section would exceed
    /// the maximum allowable length. ([`u16::MAX`]` - 1`)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::xiv_macro::Macro;
    /// use libxivdat::xiv_macro::icon::MacroIcon;
    ///
    /// let a_macro = Macro::new(
    ///     "Title".to_string(),
    ///     vec!["Circle".to_string()],
    ///     MacroIcon::SymbolCircle
    /// ).unwrap();
    ///
    /// let bytes = a_macro.as_bytes();
    /// assert!(bytes.is_ok());
    /// ```
    pub fn as_bytes(&self) -> Result<Vec<u8>, DATError> {
        let sections = self.as_section_data()?;
        let mut byte_vec = Vec::<u8>::new();
        for section in sections.into_iter() {
            let mut sec_bytes = Vec::<u8>::from(section);
            byte_vec.append(&mut sec_bytes);
        }
        Ok(byte_vec)
    }

    /// Returns a [`Vec`] of [`SectionData`](crate::section::SectionData) representing the
    /// [`MacroData`].
    ///
    /// # Errors
    ///
    /// Returns a [`DATError::ContentOverflow`] if the content of a section would exceed
    /// the maximum allowable length. ([`u16::MAX`]` - 1`)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::xiv_macro::MacroData;
    /// use libxivdat::xiv_macro::icon::MacroIcon;
    ///
    /// let a_macro = MacroData::new(
    ///     "Title",
    ///     vec!["Circle"],
    ///     &MacroIcon::SymbolCircle
    /// ).unwrap();
    ///
    /// let sections = a_macro.as_section_data().unwrap();
    ///
    /// assert_eq!(sections[0].content, "Title");
    /// ```
    pub fn as_section_data(&self) -> Result<Vec<SectionData>, DATError> {
        let mut sec_vec = vec![
            SectionData::new(SECTION_TAG_TITLE, self.title)?,
            SectionData::new(SECTION_TAG_ICON, self.icon_id)?,
            SectionData::new(SECTION_TAG_KEY, self.icon_key)?,
        ];
        for line in self.lines.iter() {
            sec_vec.push(SectionData::new(SECTION_TAG_LINE, line)?);
        }
        Ok(sec_vec)
    }

    /// Changes the [`icon_key`](Self.icon_key) and [`icon_id`](Self.icon_id) to a valid pair based on an input
    /// [`MacroIcon`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::xiv_macro::MacroData;
    /// use libxivdat::xiv_macro::icon::MacroIcon;
    ///
    /// let mut a_macro = MacroData::new(
    ///     "Title",
    ///     vec![""],
    ///     &MacroIcon::NoIcon
    /// ).unwrap();
    ///
    /// assert_eq!(a_macro.icon_id, "0000000");
    /// assert_eq!(a_macro.icon_key, "000");
    ///
    /// a_macro.change_icon(&MacroIcon::SymbolArrowUp);
    /// assert_eq!(a_macro.icon_id, "00102FF");
    /// assert_eq!(a_macro.icon_key, "037");
    /// ```
    pub fn change_icon(&mut self, icon: &'a MacroIcon) -> () {
        let (key, id) = macro_icon_to_key_and_id(&icon);
        self.icon_key = key;
        self.icon_id = id;
    }

    /// Builds a [`MacroData`] from a [`Vec`] of [`SectionData`](crate::section::SectionData).
    /// The expected pattern of section tags is "T" (Title), "I" (Icon), "K", (Key), and repeating "L"s (Lines).
    /// Valid macros always contain exactly 15 lines, even if their contents are blank. This function checks
    /// the data for validity, unlick [`from_sections_unsafe()`](Self::from_sections_unsafe)
    ///
    /// This is equivalent to calling [`from_section_data_unsafe()`](Self::from_section_data_unsafe) followed by
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
    /// Returns [`DATError::ContentOverflow`] if the title or any line is too long, or if there are too many lines.
    ///
    /// Returns [`DATError::ContentUnderflow`] if there are too few lines.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::section::SectionData;
    /// use libxivdat::xiv_macro::MacroData;
    ///
    /// let mut sections = vec![
    ///     SectionData { content: "Title", content_size: 6, tag: "T" },
    ///     SectionData { content: "0000000", content_size: 8, tag: "I" },
    ///     SectionData { content: "000", content_size: 4, tag: "K" }
    /// ];
    /// for line in std::iter::repeat("").take(15) {
    ///     sections.push(SectionData { content: line, content_size: 1, tag: "L" });
    /// }
    /// let result_macro = MacroData::from_sections(sections).unwrap();
    ///
    /// assert_eq!(result_macro.title, "Title");
    /// ```
    pub fn from_sections(sections: Vec<SectionData>) -> Result<MacroData, DATError> {
        let res_macro = Self::from_sections_unsafe(sections)?;
        if let Some(validation_err) = res_macro.validate() {
            Err(validation_err)
        } else {
            Ok(res_macro)
        }
    }

    /// Builds a [`MacroData`] from a [`Vec`] of [`SectionData`](crate::section::SectionData).
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
    /// use libxivdat::section::SectionData;
    /// use libxivdat::xiv_macro::MacroData;
    ///
    /// let sections = vec![
    ///     SectionData { content: "Title", content_size: 6, tag: "T" },
    ///     SectionData { content: "0000000", content_size: 8, tag: "I" },
    ///     SectionData { content: "000", content_size: 4, tag: "K" },
    ///     SectionData { content: "A one line macro!?", content_size: 19, tag: "L" }
    /// ];
    /// let result_macro = MacroData::from_sections_unsafe(sections).unwrap();
    ///
    /// assert_eq!(result_macro.title, "Title");
    /// assert_eq!(result_macro.lines.len(), 1);
    /// ```
    pub fn from_sections_unsafe(sections: Vec<SectionData>) -> Result<MacroData, DATError> {
        if sections.len() < 4 {
            return Err(DATError::InvalidInput("Macros require a minimum of 4 sections."));
        }
        if sections[0].tag != SECTION_TAG_TITLE {
            return Err(DATError::InvalidInput("First section was not a Title (T) section."));
        }
        let title = sections[0].content;
        if sections[1].tag != SECTION_TAG_ICON {
            return Err(DATError::InvalidInput("Second section was not a Icon (I) section."));
        }
        let icon_id = sections[1].content;
        if sections[2].tag != SECTION_TAG_KEY {
            return Err(DATError::InvalidInput("Third section was not a Key (K) section."));
        }
        let icon_key = sections[2].content;
        let mut lines = Vec::<&str>::new();
        for line in sections[3..].iter() {
            if line.tag != "L" {
                return Err(DATError::InvalidInput("Non-line (L) section in lines block."));
            }
            lines.push(line.content);
        }
        Ok(MacroData {
            icon_key,
            icon_id,
            lines,
            title,
        })
    }

    /// Gets the [`MacroIcon`] correpsonding to the current [`icon_key`](Self.icon_key) and [`icon_id`](Self.icon_id).
    /// Returns [`None`] if the id and key do not correspond to a known valid icon.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::xiv_macro::MacroData;
    /// use libxivdat::xiv_macro::icon::MacroIcon;
    ///
    /// let a_macro = MacroData::new(
    ///     "Title",
    ///     vec![""],
    ///     &MacroIcon::SymbolArrowUp
    /// ).unwrap();
    /// assert_eq!(a_macro.get_icon().unwrap(), MacroIcon::SymbolArrowUp);
    /// ```
    pub fn get_icon(&self) -> Option<MacroIcon> {
        macro_icon_from_key_and_id(&self.icon_key, &self.icon_id)
    }

    /// Builds a new [`MacroData`] with a given title, [`MacroIcon`], and content.
    /// This ensures that the macro meets the spec described below, which is used
    /// by the game client. If the provided line count is less than 15, the count
    /// will be padded with blank lines.
    ///
    /// To create an unvalidated [`MacroData`], you can directly
    /// instantiate a struct literal.
    ///
    /// # Macro spec
    ///
    /// Title: No more than 20 utf-8 characters.
    /// Icon: Icon id and key are a matching pair corresponding to a valid [`MacroIcon`].
    /// Lines: Exactly 15 lines of no more than 180 utf-8 characters.
    ///
    /// # Errors
    ///
    /// Returns [`DATError::ContentOverflow`] if the title or content are too long, or if there
    /// are too many lines.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::xiv_macro::MacroData;
    /// use libxivdat::xiv_macro::icon::MacroIcon;
    ///
    /// let a_macro = MacroData::new(
    ///     "Title",
    ///     vec!["Circle"],
    ///     &MacroIcon::SymbolCircle
    /// ).unwrap();
    ///
    /// assert_eq!(a_macro.title, "Title");
    /// assert_eq!(a_macro.lines[0], "Circle");
    /// assert_eq!(a_macro.get_icon().unwrap(), MacroIcon::SymbolCircle);
    /// ```
    pub fn new(title: &'a str, lines: Vec<&'a str>, icon: &'a MacroIcon) -> Result<MacroData<'a>, DATError> {
        let (icon_key, icon_id) = macro_icon_to_key_and_id(&icon);
        let mut padded_lines = lines.clone();
        if lines.len() < 15 {
            for line in std::iter::repeat("").take(15 - lines.len()) {
                padded_lines.push(line);
            }
        }
        let res_macro = MacroData {
            icon_id: icon_id,
            icon_key: icon_key,
            lines: padded_lines,
            title,
        };
        match res_macro.validate() {
            Some(err) => Err(err),
            None => Ok(res_macro),
        }
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
    /// use libxivdat::xiv_macro::MacroData;
    ///
    /// let a_macro = MacroData {
    ///     icon_id: "0000000",
    ///     icon_key: "000",
    ///     lines: vec![""; 15],
    ///     title: "Title"
    /// };
    /// assert!(a_macro.validate().is_none());
    /// ```
    ///
    /// ```rust
    /// use libxivdat::xiv_macro::MacroData;
    ///
    /// let a_macro = MacroData {
    ///     icon_id: "123456",
    ///     icon_key: "XYZ",
    ///     lines: vec![""; 1],
    ///     title: "Looooooooooooooooong Title"
    /// };
    /// assert!(a_macro.validate().is_some());
    /// ```
    pub fn validate(&self) -> Option<DATError> {
        if self.title.len() > 20 {
            return Some(DATError::ContentOverflow("Title is longer than 20 characters."));
        }
        if macro_icon_from_key_and_id(&self.icon_key, &self.icon_id).is_none() {
            return Some(DATError::InvalidInput("Macro icon is invalid."));
        }
        if self.lines.len() < 15 {
            return Some(DATError::ContentUnderflow("Macro has fewer than 15 lines."));
        }
        if self.lines.len() > 15 {
            return Some(DATError::ContentOverflow("Macro has more than 15 lines."));
        }
        for line in self.lines.iter() {
            if line.len() > 180 {
                return Some(DATError::ContentOverflow("Line is longer than 180 characters."));
            }
        }
        None
    }
}

/// Reads the next [`Macro`] from a [`DATFile`](crate::dat_file::DATFile).
///
/// # Errors
/// 
/// Returns [`DATError::IncorrectType`] if the file appears to be of a type other than
/// [`DATType::Macro`].
///
/// If an I/O error occurs while reading the file, a [`DATError::FileIO`](crate::dat_error::DATError::FileIO)
/// error will be returned wrapping the underlying FS error.
///
/// If the macro title or lines do not contain valid utf-8, a [`DATError::BadEncoding`]
/// error will be returned.
///
/// # Examples
/// ```rust
/// use libxivdat::dat_file::DATFile;
/// use libxivdat::xiv_macro::read_macro;
/// use libxivdat::xiv_macro::icon::MacroIcon;
///
/// let mut dat_file = DATFile::open("./resources/TEST_MACRO.DAT").unwrap();
/// let a_macro = read_macro(&mut dat_file).unwrap();
///
/// assert_eq!(a_macro.title, "0");
/// assert_eq!(a_macro.lines[0], "DefaultIcon");
/// assert_eq!(a_macro.get_icon().unwrap(), MacroIcon::DefaultIcon);
/// ```
pub fn read_macro(dat_file: &mut DATFile) -> Result<Macro, DATError> {
    if dat_file.file_type() != DATType::Macro {
        Err(DATError::IncorrectType("Attempted to read a macro from a non-macro file."))
    }
    else {
        Ok(read_macro_unsafe(dat_file)?)
    }
}

/// Reads all [`Macros`](Macro) from a specified DAT file, returning a [`Vec`] of them.
/// This performs only one read operation on the underlying file, loading the entire content into memory
/// to prevent repeat file access. This is similar to [`read_content()`](crate::dat_file::read_content),
/// but returns a `Vec<Macro>` instead of raw bytes. A valid macro file should always contain 100 macros,
/// but this function will attempt to read files of other sizes.
///
/// # Errors
/// 
/// Returns [`DATError::IncorrectType`] if the file appears to be of a type other than
/// [`DATType::Macro`].
///
/// Returns a [`DATError::ContentOverflow`](crate::dat_error::DATError::ContentOverflow) or
/// [`DATError::ContentUnderflow`](crate::dat_error::DATError::ContentUnderflow) if a macro section content block
/// does not match the expected length specified in the section header.
///
/// Returns a [`DATError::BadEncoding`](crate::dat_error::DATError::BadEncoding) if a macro title or line does not
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
pub fn read_macro_content<P: AsRef<Path>>(path: P) -> Result<Vec<Macro>, DATError> {
    if check_type(&path)? != DATType::Macro {
        Err(DATError::IncorrectType("Attempted to read a macro from a non-macro file."))
    }
    else {
        Ok(read_macro_content_unsafe(path)?)
    }
}

/// Reads the next [`Macro`] from a [`DATFile`](crate::dat_file::DATFile). This does not check that the target
/// file is a macro file.
///
/// # Errors
///
/// If an I/O error occurs while reading the file, a [`DATError::FileIO`](crate::dat_error::DATError::FileIO)
/// error will be returned wrapping the underlying FS error.
///
/// If the macro title or lines do not contain valid utf-8, a [`DATError::BadEncoding`]
/// error will be returned.
///
/// # Examples
/// ```rust
/// use libxivdat::dat_file::DATFile;
/// use libxivdat::xiv_macro::read_macro_unsafe;
/// use libxivdat::xiv_macro::icon::MacroIcon;
///
/// let mut dat_file = DATFile::open("./resources/TEST_MACRO.DAT").unwrap();
/// let a_macro = read_macro_unsafe(&mut dat_file).unwrap();
///
/// assert_eq!(a_macro.title, "0");
/// assert_eq!(a_macro.lines[0], "DefaultIcon");
/// assert_eq!(a_macro.get_icon().unwrap(), MacroIcon::DefaultIcon);
/// ```
pub fn read_macro_unsafe(dat_file: &mut DATFile) -> Result<Macro, DATError> {
    let title_sec = read_section(dat_file)?;
    let mut sec_vec = vec![title_sec];
    loop {
        let next_section = match read_section(dat_file) {
            Ok(next_section) => next_section,
            Err(err) => match err {
                DATError::EndOfFile(_) => break,
                _ => return Err(err)
            }
        };
        if next_section.tag == SECTION_TAG_TITLE {
            break;
        }
        sec_vec.push(next_section);
    }
    Ok(Macro::from_sections_unsafe(sec_vec)?)
}

/// Reads all [`Macros`](Macro) from a specified DAT file, returning a [`Vec`] of them.
/// This performs only one read operation on the underlying file, loading the entire content into memory
/// to prevent repeat file access. This is similar to [`read_content()`](crate::dat_file::read_content),
/// but returns a `Vec<Macro>` instead of raw bytes. A valid macro file should always contain 100 macros,
/// but this function will attempt to read files of other sizes. This does not check that the
/// file is a macro file.
///
/// # Errors
///
/// Returns a [`DATError::ContentOverflow`](crate::dat_error::DATError::ContentOverflow) or
/// [`DATError::ContentUnderflow`](crate::dat_error::DATError::ContentUnderflow) if a macro section content block
/// does not match the expected length specified in the section header.
///
/// Returns a [`DATError::BadEncoding`](crate::dat_error::DATError::BadEncoding) if a macro title or line does not
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
/// use libxivdat::xiv_macro::read_macro_content_unsafe;
/// use libxivdat::xiv_macro::icon::MacroIcon;
///
/// let macro_contents = read_macro_content_unsafe("./resources/TEST_MACRO.DAT").unwrap();
///
/// assert_eq!(macro_contents[0].title, "0");
/// assert_eq!(macro_contents[0].lines[0], "DefaultIcon");
/// assert_eq!(macro_contents[0].get_icon().unwrap(), MacroIcon::DefaultIcon);
///
/// assert_eq!(macro_contents[1].title, "1");
/// assert_eq!(macro_contents[1].lines[0], "DPS1");
/// assert_eq!(macro_contents[1].get_icon().unwrap(), MacroIcon::DPS1);
/// ```
pub fn read_macro_content_unsafe<P: AsRef<Path>>(path: P) -> Result<Vec<Macro>, DATError> {
    let sections = read_section_content(path)?;
    let mut macro_vec = Vec::<Macro>::new();
    let mut sec_vec = Vec::<Section>::new();
    for next_section in sections.into_iter() {
        // Push a new macro on every title
        if next_section.tag == SECTION_TAG_TITLE {
            if sec_vec.len() > 0 {
                macro_vec.push(Macro::from_sections_unsafe(sec_vec)?);
            }
            sec_vec = Vec::<Section>::new();
        }
        sec_vec.push(next_section);
    }
    Ok(macro_vec)
}
