/// Enum of all possible macro icons. This includes only the default macro icons, not icons
/// configured with the `/micon <action>` command. Internally, the macro data preserves the
/// icon chosen via GUI, ignoring the /micon command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacroIcon {
    /// Default macro icon
    DefaultIcon,
    /// DPS symbol with number 1
    DPS1,
    /// DPS symbol with number 2
    DPS2,
    /// DPS symbol with number 3
    DPS3,
    /// Tank symbol with number 1
    Tank1,
    /// Tank symbol with number 2
    Tank2,
    /// Tank symbol with number 3
    Tank3,
    /// Healer symbol with number 1
    Healer1,
    /// Healer symbol with number 2
    Healer2,
    /// Healer symbol with number 3
    Healer3,
    /// Crafter symbol with number 1 in purple
    CrafterPurple1,
    /// Crafter symbol with number 2 in purple
    CrafterPurple2,
    /// Crafter symbol with number 3 in purple
    CrafterPurple3,
    /// Crafter symbol with number 1 in yellow
    CrafterYellow1,
    /// Crafter symbol with number 2 in yellow
    CrafterYellow2,
    /// Crafter symbol with number 3 in yellow
    CrafterYellow3,
    /// Crafter symbol with number 1 in green
    CrafterGreen1,
    /// Crafter symbol with number 2 in green
    CrafterGreen2,
    /// Crafter symbol with number 3 in green
    CrafterGreen3,
    /// Hammer with gold border
    ItemHammer,
    /// Sword with gold border
    ItemSword,
    /// Shield with gold border
    ItemShield,
    /// Ring with gold border
    ItemRing,
    /// Shoes with gold border
    ItemShoes,
    /// Hat with gold border
    ItemHat,
    /// Bottle with gold border
    ItemBottle,
    /// Bread with gold border
    ItemBread,
    /// Gather symbol with number 1
    Gatherer1,
    /// Gather symbol with number 2
    Gatherer2,
    /// Gather symbol with number 3
    Gatherer3,
    /// The number 0 in gold on a blue background
    Number0,
    /// The number 1 in gold on a blue background
    Number1,
    /// The number 2 in gold on a blue background
    Number2,
    /// The number 3 in gold on a blue background
    Number3,
    /// The number 4 in gold on a blue background
    Number4,
    /// The number 5 in gold on a blue background
    Number5,
    /// The number 6 in gold on a blue background
    Number6,
    /// The number 7 in gold on a blue background
    Number7,
    /// The number 8 in gold on a blue background
    Number8,
    /// The number 9 in gold on a blue background
    Number9,
    /// The number 10 in gold on a blue background
    Number10,
    /// The number 0 in blue on a gold background
    InverseNumber0,
    /// The number 1 in blue on a gold background
    InverseNumber1,
    /// The number 2 in blue on a gold background
    InverseNumber2,
    /// The number 3 in blue on a gold background
    InverseNumber3,
    /// The number 4 in blue on a gold background
    InverseNumber4,
    /// The number 5 in blue on a gold background
    InverseNumber5,
    /// The number 6 in blue on a gold background
    InverseNumber6,
    /// The number 7 in blue on a gold background
    InverseNumber7,
    /// The number 8 in blue on a gold background
    InverseNumber8,
    /// The number 9 in blue on a gold background
    InverseNumber9,
    /// The number 10 in blue on a gold background
    InverseNumber10,
    /// A gray left arrow symbol
    SymbolArrowLeft,
    /// A gray right arrow symbol
    SymbolArrowRight,
    /// A gray up arrow symbol
    SymbolArrowUp,
    /// A gray down arrow symbol
    SymbolArrowDown,
    /// A gray circle symbol
    SymbolCircle,
    /// A gray triangle symbol
    SymbolTriangle,
    /// A gray square symbol
    SymbolSquare,
    /// A gray x symbol
    SymbolX,
    /// A gray no symbol
    SymbolNo,
    /// A gray warning symbol
    SymbolWarning,
    /// A gray check mark symbol
    SymbolCheck,
    /// A gray star symbol
    SymbolStar,
    /// A gray question mark symbol
    SymbolQuestion,
    /// A gray exclamation mark symbol
    SymbolExclamation,
    /// A gray plus symbol
    SymbolPlus,
    /// A gray minus symbol
    SymbolMinus,
    /// A gray clock symbol
    SymbolClock,
    /// A gray light bulb symbol
    SymbolBulb,
    /// A gray cog wheel / settings symbol
    SymbolCog,
    /// A gray magnifying glass / search symbol
    SymbolSearch,
    /// A gray speech bubble symbol
    SymbolSpeech,
    /// A gray heart symbol
    SymbolHeart,
    /// A gray spade symbol
    SymbolSpade,
    /// A gray club symbol
    SymbolClub,
    /// A gray diamond symbol
    SymbolDiamond,
    /// A gray dice symbol
    SymbolDice,
    /// A fire crystal item icon
    CrystalFire,
    /// An ice crystal item icon
    CrystalIce,
    /// A wind crystal item icon
    CrystalWind,
    /// An earth crystal item icon
    CrystalEarth,
    /// A lightning crystal item icon
    CrystalLightning,
    /// A water crystal item icon
    CrystalWater,
    /// A valid macro with no icon; this is technically possible, although the gui doesn't allow it
    NoIcon,
}

/// Returns the [`MacroIcon`] corresponding to the raw values of the key and icon
/// [`Sections`](crate::section::Section) of a macro.
///
/// `None` is only returned if the provided key and id values are completely invalid.
/// Values of "000" and "0000000" (used for undefined macros) are considered valid and return
/// [`MacroIcon::NoIcon`].
///
/// # Examples
/// ```rust
/// use libxivdat::xiv_macro::icon::{MacroIcon,macro_icon_from_key_and_id};
///
/// let key_val = "014";
/// let id_val = "00005E9";
/// assert_eq!(macro_icon_from_key_and_id(key_val, id_val).unwrap(), MacroIcon::ItemHammer);
/// ```
pub fn macro_icon_from_key_and_id(key: &str, id: &str) -> Option<MacroIcon> {
    match (key, id) {
        ("000", "0000000") => Some(MacroIcon::NoIcon),
        ("001", "00101D1") => Some(MacroIcon::DefaultIcon),
        ("002", "0010235") => Some(MacroIcon::DPS1),
        ("003", "0010236") => Some(MacroIcon::DPS1),
        ("004", "0010237") => Some(MacroIcon::DPS1),
        ("005", "0010249") => Some(MacroIcon::Tank1),
        ("006", "001024A") => Some(MacroIcon::Tank2),
        ("007", "001024B") => Some(MacroIcon::Tank3),
        ("008", "001025D") => Some(MacroIcon::Healer1),
        ("009", "001025E") => Some(MacroIcon::Healer2),
        ("00A", "001025F") => Some(MacroIcon::Healer3),
        ("00B", "00101E5") => Some(MacroIcon::CrafterPurple1),
        ("00C", "00101E6") => Some(MacroIcon::CrafterPurple2),
        ("00D", "00101E7") => Some(MacroIcon::CrafterPurple3),
        ("00E", "00101F9") => Some(MacroIcon::CrafterYellow1),
        ("00F", "00101FA") => Some(MacroIcon::CrafterYellow2),
        ("010", "00101FB") => Some(MacroIcon::CrafterYellow3),
        ("011", "001020D") => Some(MacroIcon::CrafterGreen1),
        ("012", "001020E") => Some(MacroIcon::CrafterGreen2),
        ("013", "001020F") => Some(MacroIcon::CrafterGreen3),
        ("014", "00005E9") => Some(MacroIcon::ItemHammer),
        ("015", "000061B") => Some(MacroIcon::ItemSword),
        ("016", "000064D") => Some(MacroIcon::ItemShield),
        ("017", "000067E") => Some(MacroIcon::ItemRing),
        ("018", "00006B1") => Some(MacroIcon::ItemShoes),
        ("019", "00006E2") => Some(MacroIcon::ItemHat),
        ("01A", "0000715") => Some(MacroIcon::ItemBottle),
        ("01B", "0000746") => Some(MacroIcon::ItemBread),
        ("01C", "0010221") => Some(MacroIcon::Gatherer1),
        ("01D", "0010222") => Some(MacroIcon::Gatherer2),
        ("01E", "0010223") => Some(MacroIcon::Gatherer3),
        ("01F", "0010271") => Some(MacroIcon::Number0),
        ("020", "0010272") => Some(MacroIcon::Number1),
        ("021", "0010273") => Some(MacroIcon::Number2),
        ("022", "0010274") => Some(MacroIcon::Number3),
        ("023", "0010275") => Some(MacroIcon::Number4),
        ("024", "0010276") => Some(MacroIcon::Number5),
        ("025", "0010277") => Some(MacroIcon::Number6),
        ("026", "0010278") => Some(MacroIcon::Number7),
        ("027", "0010279") => Some(MacroIcon::Number8),
        ("028", "001027A") => Some(MacroIcon::Number9),
        ("029", "001027B") => Some(MacroIcon::Number10),
        ("02A", "0010285") => Some(MacroIcon::InverseNumber0),
        ("02B", "0010286") => Some(MacroIcon::InverseNumber1),
        ("02C", "0010287") => Some(MacroIcon::InverseNumber2),
        ("02D", "0010288") => Some(MacroIcon::InverseNumber3),
        ("02E", "0010289") => Some(MacroIcon::InverseNumber4),
        ("02F", "001028A") => Some(MacroIcon::InverseNumber5),
        ("030", "001028B") => Some(MacroIcon::InverseNumber6),
        ("031", "001028C") => Some(MacroIcon::InverseNumber7),
        ("032", "001028D") => Some(MacroIcon::InverseNumber8),
        ("033", "001028E") => Some(MacroIcon::InverseNumber9),
        ("034", "001028F") => Some(MacroIcon::InverseNumber10),
        ("035", "00102FD") => Some(MacroIcon::SymbolArrowLeft),
        ("036", "00102FE") => Some(MacroIcon::SymbolArrowRight),
        ("037", "00102FF") => Some(MacroIcon::SymbolArrowUp),
        ("038", "0010300") => Some(MacroIcon::SymbolArrowDown),
        ("039", "0010301") => Some(MacroIcon::SymbolCircle),
        ("03A", "0010302") => Some(MacroIcon::SymbolTriangle),
        ("03B", "0010303") => Some(MacroIcon::SymbolSquare),
        ("03C", "0010304") => Some(MacroIcon::SymbolX),
        ("03D", "0010305") => Some(MacroIcon::SymbolNo),
        ("03E", "0010306") => Some(MacroIcon::SymbolWarning),
        ("03F", "0010307") => Some(MacroIcon::SymbolCheck),
        ("040", "0010308") => Some(MacroIcon::SymbolStar),
        ("041", "0010309") => Some(MacroIcon::SymbolQuestion),
        ("042", "001030A") => Some(MacroIcon::SymbolExclamation),
        ("043", "001030B") => Some(MacroIcon::SymbolPlus),
        ("044", "001030C") => Some(MacroIcon::SymbolMinus),
        ("045", "001030D") => Some(MacroIcon::SymbolClock),
        ("046", "001030E") => Some(MacroIcon::SymbolBulb),
        ("047", "001030F") => Some(MacroIcon::SymbolCog),
        ("048", "0010310") => Some(MacroIcon::SymbolSearch),
        ("049", "0010311") => Some(MacroIcon::SymbolSpeech),
        ("04A", "0010312") => Some(MacroIcon::SymbolHeart),
        ("04B", "0010313") => Some(MacroIcon::SymbolSpade),
        ("04C", "0010314") => Some(MacroIcon::SymbolClub),
        ("04D", "0010315") => Some(MacroIcon::SymbolDiamond),
        ("04E", "0010316") => Some(MacroIcon::SymbolDice),
        ("04F", "0004E27") => Some(MacroIcon::CrystalFire),
        ("050", "0004E29") => Some(MacroIcon::CrystalIce),
        ("051", "0004E2A") => Some(MacroIcon::CrystalWind),
        ("052", "0004E2C") => Some(MacroIcon::CrystalEarth),
        ("053", "0004E2B") => Some(MacroIcon::CrystalLightning),
        ("054", "0004E28") => Some(MacroIcon::CrystalWater),
        _ => None,
    }
}

/// Returns the key and id [`Section`](crate::section::Section) contents
/// corresponding to a [`MacroIcon`].
///
/// # Examples
/// ```rust
/// use libxivdat::xiv_macro::icon::{MacroIcon,macro_icon_to_key_and_id};
///
/// let (key_val, id_val) = macro_icon_to_key_and_id(&MacroIcon::ItemHammer);
/// assert_eq!(key_val, "014");
/// assert_eq!(id_val, "00005E9");
/// ```
pub fn macro_icon_to_key_and_id(macro_icon: &MacroIcon) -> (&str, &str) {
    match macro_icon {
        MacroIcon::NoIcon => ("000", "0000000"),
        MacroIcon::DefaultIcon => ("001", "00101D1"),
        MacroIcon::DPS1 => ("002", "0010235"),
        MacroIcon::DPS2 => ("003", "0010236"),
        MacroIcon::DPS3 => ("004", "0010237"),
        MacroIcon::Tank1 => ("005", "0010249"),
        MacroIcon::Tank2 => ("006", "001024A"),
        MacroIcon::Tank3 => ("007", "001024B"),
        MacroIcon::Healer1 => ("008", "001025D"),
        MacroIcon::Healer2 => ("009", "001025E"),
        MacroIcon::Healer3 => ("00A", "001025F"),
        MacroIcon::CrafterPurple1 => ("00B", "00101E5"),
        MacroIcon::CrafterPurple2 => ("00C", "00101E6"),
        MacroIcon::CrafterPurple3 => ("00D", "00101E7"),
        MacroIcon::CrafterYellow1 => ("00E", "00101F9"),
        MacroIcon::CrafterYellow2 => ("00F", "00101FA"),
        MacroIcon::CrafterYellow3 => ("010", "00101FB"),
        MacroIcon::CrafterGreen1 => ("011", "001020D"),
        MacroIcon::CrafterGreen2 => ("012", "001020E"),
        MacroIcon::CrafterGreen3 => ("013", "001020F"),
        MacroIcon::ItemHammer => ("014", "00005E9"),
        MacroIcon::ItemSword => ("015", "000061B"),
        MacroIcon::ItemShield => ("016", "000064D"),
        MacroIcon::ItemRing => ("017", "000067E"),
        MacroIcon::ItemShoes => ("018", "00006B1"),
        MacroIcon::ItemHat => ("019", "00006E2"),
        MacroIcon::ItemBottle => ("01A", "0000715"),
        MacroIcon::ItemBread => ("01B", "0000746"),
        MacroIcon::Gatherer1 => ("01C", "0010221"),
        MacroIcon::Gatherer2 => ("01D", "0010222"),
        MacroIcon::Gatherer3 => ("01E", "0010223"),
        MacroIcon::Number0 => ("01F", "0010271"),
        MacroIcon::Number1 => ("020", "0010272"),
        MacroIcon::Number2 => ("021", "0010273"),
        MacroIcon::Number3 => ("022", "0010274"),
        MacroIcon::Number4 => ("023", "0010275"),
        MacroIcon::Number5 => ("024", "0010276"),
        MacroIcon::Number6 => ("025", "0010277"),
        MacroIcon::Number7 => ("026", "0010278"),
        MacroIcon::Number8 => ("027", "0010279"),
        MacroIcon::Number9 => ("028", "001027A"),
        MacroIcon::Number10 => ("029", "001027B"),
        MacroIcon::InverseNumber0 => ("02A", "0010285"),
        MacroIcon::InverseNumber1 => ("02B", "0010286"),
        MacroIcon::InverseNumber2 => ("02C", "0010287"),
        MacroIcon::InverseNumber3 => ("02D", "0010288"),
        MacroIcon::InverseNumber4 => ("02E", "0010289"),
        MacroIcon::InverseNumber5 => ("02F", "001028A"),
        MacroIcon::InverseNumber6 => ("030", "001028B"),
        MacroIcon::InverseNumber7 => ("031", "001028C"),
        MacroIcon::InverseNumber8 => ("032", "001028D"),
        MacroIcon::InverseNumber9 => ("033", "001028E"),
        MacroIcon::InverseNumber10 => ("034", "001028F"),
        MacroIcon::SymbolArrowLeft => ("035", "00102FD"),
        MacroIcon::SymbolArrowRight => ("036", "00102FE"),
        MacroIcon::SymbolArrowUp => ("037", "00102FF"),
        MacroIcon::SymbolArrowDown => ("038", "0010300"),
        MacroIcon::SymbolCircle => ("039", "0010301"),
        MacroIcon::SymbolTriangle => ("03A", "0010302"),
        MacroIcon::SymbolSquare => ("03B", "0010303"),
        MacroIcon::SymbolX => ("03C", "0010304"),
        MacroIcon::SymbolNo => ("03D", "0010305"),
        MacroIcon::SymbolWarning => ("03E", "0010306"),
        MacroIcon::SymbolCheck => ("03F", "0010307"),
        MacroIcon::SymbolStar => ("040", "0010308"),
        MacroIcon::SymbolQuestion => ("041", "0010309"),
        MacroIcon::SymbolExclamation => ("042", "001030A"),
        MacroIcon::SymbolPlus => ("043", "001030B"),
        MacroIcon::SymbolMinus => ("044", "001030C"),
        MacroIcon::SymbolClock => ("045", "001030D"),
        MacroIcon::SymbolBulb => ("046", "001030E"),
        MacroIcon::SymbolCog => ("047", "001030F"),
        MacroIcon::SymbolSearch => ("048", "0010310"),
        MacroIcon::SymbolSpeech => ("049", "0010311"),
        MacroIcon::SymbolHeart => ("04A", "0010312"),
        MacroIcon::SymbolSpade => ("04B", "0010313"),
        MacroIcon::SymbolClub => ("04C", "0010314"),
        MacroIcon::SymbolDiamond => ("04D", "0010315"),
        MacroIcon::SymbolDice => ("04E", "0010316"),
        MacroIcon::CrystalFire => ("04F", "0004E27"),
        MacroIcon::CrystalIce => ("050", "0004E29"),
        MacroIcon::CrystalWind => ("051", "0004E2A"),
        MacroIcon::CrystalEarth => ("052", "0004E2C"),
        MacroIcon::CrystalLightning => ("053", "0004E2B"),
        MacroIcon::CrystalWater => ("054", "0004E28"),
    }
}
