/// Contains [`MacroIcon`]
pub mod icon;

/// Number of [`Sections`](crate::section::Section) per macro. See [`MACRO_SECTION_TAG_PATTERN`] for specific sections expected.
pub const MACRO_SECTION_SIZE: usize = 18;
/// Expected pattern of [`Sections`](crate::section::Section) by tag. A single [`Macro`] is comprised of these sections.
pub const MACRO_SECTION_TAG_PATTERN: [&str; 18] = [
    "T", "I", "K", "L", "L", "L", "L", "L", "L", "L", "L", "L", "L", "L", "L", "L", "L", "L",
];

/// Data struct for a single macro from a MACRO or MACROSYS DAT file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Macro<'a> {
    icon: &'a str,
    key: &'a str,
    lines: &'a [&'a str; 15],
    title: &'a str,
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
    /// Tag: "L"
    Line = 'L' as isize,
    /// The title of a macro. The FFXIV game client limits macro titles to no more
    /// than 20 utf8 characters. This is a character limit, not a byte limit.
    /// Tag: "T"
    Title = 'T' as isize,
}
