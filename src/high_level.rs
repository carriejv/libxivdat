use crate::dat_error::DATError;

/// Defines a high-level data struct that can be represented as a writeable byte vector.
pub trait AsBytes {
    /// Returns a byte vector representing the struct. This can then be written
    /// back to the DAT file on disk.
    ///
    /// # Errors
    ///
    /// Returns a [`DATError::ContentOverflow`] if the content of a data block
    /// would exceed the maximum allowable length.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::high_level::AsBytes;
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
    fn as_bytes(&self) -> Result<Vec<u8>, DATError>;
}

/// Defines a high-level data struct that can be validated against a spec used by the game client.
pub trait Validate {
    /// Validates the struct data against the spec expected by the game client.
    /// Returns a [`DATError`] describing the error if validation fails, or [`None`]
    /// if validation is successful.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libxivdat::high_level::Validate;
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
    /// use libxivdat::high_level::Validate;
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
    fn validate(&self) -> Option<DATError>;
}