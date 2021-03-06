/// Enumeration of known FFXIV DAT file types.
/// The value of each element represents the first 4 header bytes as a little-endian i32.
/// These bytes are known static values that differentiate file types.
///
/// File types may be referenced using a human readable descriptor -- `DATType::GoldSaucer` --
/// or the filename used by FFXIV -- `DATType::GS`. These methods are interchangable and considered
/// equivalent. `DATType::GoldSaucer == DATType::GS`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DATType {
    /// GEARSET.DAT
    Gearset = 0x006b0005,
    /// GS.DAT
    GoldSaucer = 0x0067000A,
    /// HOTBAR.DAT
    Hotbar = 0x00040002,
    /// ITEMFDR.DAT
    ItemFinder = 0x00CA0008,
    /// ITEMODR.DAT
    ItemOrder = 0x00670007,
    /// KEYBIND.DAT
    Keybind = 0x00650003,
    /// LOGFLTR.DAT
    LogFilter = 0x00030004,
    /// MACRO.DAT (Character) & MACROSYS.DAT (Global)
    Macro = 0x00020001,
    /// ACQ.DAT (Acquaintances?)
    RecentTells = 0x00640006,
    /// UISAVE.DAT
    UISave = 0x00010009,
    Unknown = 0,
}

/// Provides alises matching exact file names rather than human-readable descriptors.
impl DATType {
    pub const ACQ: DATType = DATType::RecentTells;
    pub const GEARSET: DATType = DATType::Gearset;
    pub const GS: DATType = DATType::GoldSaucer;
    pub const ITEMFDR: DATType = DATType::ItemFinder;
    pub const ITEMODR: DATType = DATType::ItemOrder;
    pub const KEYBIND: DATType = DATType::Keybind;
    pub const LOGFLTR: DATType = DATType::LogFilter;
    pub const MACRO: DATType = DATType::Macro;
    pub const MACROSYS: DATType = DATType::Macro;
    pub const UISAVE: DATType = DATType::UISave;
}

impl From<u32> for DATType {
    fn from(x: u32) -> DATType {
        match x {
            x if x == DATType::Gearset as u32 => DATType::Gearset,
            x if x == DATType::GoldSaucer as u32 => DATType::GoldSaucer,
            x if x == DATType::Hotbar as u32 => DATType::Hotbar,
            x if x == DATType::ItemFinder as u32 => DATType::ItemFinder,
            x if x == DATType::ItemOrder as u32 => DATType::ItemOrder,
            x if x == DATType::Keybind as u32 => DATType::Keybind,
            x if x == DATType::LogFilter as u32 => DATType::LogFilter,
            x if x == DATType::Macro as u32 => DATType::Macro,
            x if x == DATType::RecentTells as u32 => DATType::RecentTells,
            x if x == DATType::UISave as u32 => DATType::UISave,
            _ => DATType::Unknown,
        }
    }
}

/// Gets the XOR mask used for the contents of a binary DAT file.
/// The mask is applied to only the file data content, not the header, footer, or padding null bytes.
/// Returns `None` if the file is of unknown type or does not have a mask.
///
/// # Examples
/// ```rust
/// use libxivdat::dat_type::{DATType, get_mask_for_type};
///
/// let mask = get_mask_for_type(&DATType::Macro).unwrap();
/// # let mut raw_macro_bytes = [0u8; 1];
/// for byte in raw_macro_bytes.iter_mut() {
///    *byte = *byte ^ mask;
/// }
/// ```
pub fn get_mask_for_type(file_type: &DATType) -> Option<u8> {
    match file_type {
        DATType::Gearset
        | DATType::GoldSaucer
        | DATType::ItemFinder
        | DATType::ItemOrder
        | DATType::Keybind
        | DATType::Macro
        | DATType::RecentTells => Some(0x73),
        DATType::Hotbar | DATType::UISave => Some(0x31),
        DATType::LogFilter => Some(0x00),
        _ => None,
    }
}

/// Gets the default header ending byte for a given DAT type.
/// The purpose of this value is unknown, but it is a fixed value based on file type.
/// Returns `None` if the file is of unknown type.
///
/// # Examples
/// ```rust
/// use libxivdat::dat_type::{DATType, get_default_end_byte_for_type};
/// let end_byte = get_default_end_byte_for_type(&DATType::Macro);
/// ```
pub fn get_default_end_byte_for_type(file_type: &DATType) -> Option<u8> {
    match file_type {
        DATType::Gearset
        | DATType::GoldSaucer
        | DATType::ItemFinder
        | DATType::ItemOrder
        | DATType::Keybind
        | DATType::Macro
        | DATType::RecentTells => Some(0xFF),
        DATType::Hotbar => Some(0x31),
        DATType::LogFilter => Some(0x00),
        DATType::UISave => Some(0x21),
        _ => None,
    }
}

/// Gets the default maximum content size of a DAT file for a given type.
/// Returns `None` if the file is of unknown type or has no standard size.
///
/// # Examples
/// ```rust
/// use libxivdat::dat_type::{DATType, get_default_max_size_for_type};
/// let max_size = get_default_max_size_for_type(&DATType::Macro).unwrap();
/// ```
pub fn get_default_max_size_for_type(file_type: &DATType) -> Option<u32> {
    match file_type {
        DATType::Gearset => Some(44849),
        DATType::GoldSaucer => Some(649),
        DATType::Hotbar => Some(204800),
        DATType::ItemFinder => Some(14030),
        DATType::ItemOrder => Some(15193),
        DATType::Keybind => Some(20480),
        DATType::LogFilter => Some(2048),
        DATType::Macro => Some(286720),
        DATType::RecentTells => Some(2048),
        DATType::UISave => Some(64512),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Read;

    const FILE_TYPE_MAP: [(DATType, &str); 9] = [
        (DATType::ACQ, "./resources/default_dats/ACQ.DAT"),
        (DATType::GEARSET, "./resources/default_dats/GEARSET.DAT"),
        (DATType::GS, "./resources/default_dats/GS.DAT"),
        (DATType::ITEMFDR, "./resources/default_dats/ITEMFDR.DAT"),
        (DATType::ITEMODR, "./resources/default_dats/ITEMODR.DAT"),
        (DATType::KEYBIND, "./resources/default_dats/KEYBIND.DAT"),
        (DATType::LOGFLTR, "./resources/default_dats/LOGFLTR.DAT"),
        (DATType::MACRO, "./resources/default_dats/MACRO.DAT"),
        (DATType::UISAVE, "./resources/default_dats/UISAVE.DAT"),
    ];

    #[test]
    fn test_from_header_bytes() -> Result<(), String> {
        for case in FILE_TYPE_MAP.iter() {
            let mut file = match File::open(case.1) {
                Ok(file) => file,
                Err(err) => return Err(format!("Error opening file: {}", err)),
            };
            let mut buf = [0u8; 4];
            match file.read(&mut buf) {
                Ok(_) => (),
                Err(err) => return Err(format!("Error reading file: {}", err)),
            };
            let id_bytes = u32::from_le_bytes(buf);
            assert_eq!(DATType::from(id_bytes), case.0);
        }
        Ok(())
    }

    #[test]
    fn test_get_default_end_byte_for_type() -> Result<(), String> {
        for case in FILE_TYPE_MAP.iter() {
            match get_default_end_byte_for_type(&case.0) {
                Some(_) => (),
                None => return Err(format!("No value returned for case {}.", case.1)),
            };
        }
        Ok(())
    }

    #[test]
    fn test_get_default_max_size_for_type() -> Result<(), String> {
        for case in FILE_TYPE_MAP.iter() {
            match get_default_max_size_for_type(&case.0) {
                Some(_) => (),
                None => return Err(format!("No value returned for case {}.", case.1)),
            };
        }
        Ok(())
    }

    #[test]
    fn test_get_mask_for_type() -> Result<(), String> {
        for case in FILE_TYPE_MAP.iter() {
            match get_mask_for_type(&case.0) {
                Some(_) => (),
                None => return Err(format!("No value returned for case {}.", case.1)),
            };
        }
        Ok(())
    }
}
