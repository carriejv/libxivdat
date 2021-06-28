# libxivdat

A library for working with Final Fantasy XIV .DAT files. These files store client-side game config including macros, hotkeys, and ui settings.

This is still a work in progress and currently provides only low-level file io capabilities. See [future plans](#future-plans) for more info.

## DATFile

`DATFile` provides a common, general-purpose method of working with binary DAT files. `DATFile` emulates Rust's std lib `File` but reads and writes only from the inner content block of .DAT files, automatically handling header updates, padding, and masking as necessary.

### Examples

```rust
extern crate libxivdat;
use libxivdat::dat_file::{DATFile,DATType};

let mut dat_file = DATFile::open("GEARSET.DAT")?;
if dat_file.file_type == DATType::Gearset {
    let mut gs_1_bytes = [0u8; 444];
    dat_file.read(&gs_1_bytes)?;
}
else {
     panic!("Not a gearset file!");
}
```

## DAT Data Content

Most DAT files (excluding those marked as "Unique" in the support table), share a common file structure consisting of a header, content block, and footer.

Internally, some DAT file content blocks use a variable-length data structure referred to as a `Section` in this library. A Section consists of a single UTF-8 char type tag, u8 size, and a null-terminated UTF-8 string. A single resource (ie, a macro) is then comprised of a repeating pattern of Sections.

Other DAT files use fixed-size resource blocks, with each resource immediately following the last. These are referred to as "Block DATs" below.

## Plaintext DAT files

Some DAT files (namely `COMMON.DAT`, `CONTROL0.DAT`, and `CONTROL1.DAT`) created by FF XIV are actually just UTF-8 plaintext and do not share a common format with the binary DAT files.

Support for working with plaintext DATs may happen at some future point, but isn't currently a priority.

## Future Plans

The goal of this library is to fully abstract the data structures of DAT files to allow interacting with the files via structured data as opposed to raw byte streams. Each file has its own internal data types, so these layers will be implemented one at a time as optional features.

My focus with this libary is mainly on macros, gearsets, and ui config. Fully abstracted support for other DAT types will liklely be a long time coming unless you build it yourself.

## DAT Type Support

| File               | Contains                         | Type       | DATFile Read/Write | High Level Structs |
|--------------------|----------------------------------|------------|--------------------|--------------------|
| ACQ.DAT            | Recent /tell history             | Section    |          ✔️        |          ✗         |
| ADDON.DAT          | ?                                | Unique     |          ✗         |          ✗         |
| COMMON.DAT         | Character configuration          | Plaintext  |          ✗         |          ✗         |
| CONTROL0.DAT       | Gamepad control config           | Plaintext  |          ✗         |          ✗         |
| CONTROL1.DAT       | Keyboard/mouse control config    | Plaintext  |          ✗         |          ✗         |
| FFXIV_CHARA_XX.DAT | Character appearance presets     | Unique     |          ✗         |          ✗         |
| GEARSET.DAT        | Gearsets                         | Block      |          ✔️        |          ✗         |
| GS.DAT             | Gold Saucer config (Triad decks) | Block      |          ✔️        |          ✗         |
| HOTBAR.DAT         | Hotbar layouts                   | Block      |          ✔️        |          ✗         |
| ITEMFDR.DAT        | "Search for item" indexing?      | Block      |          ✔️        |          ✗         |
| ITEMODR.DAT        | Item order in bags               | Block      |          ✔️        |          ✗         |
| KEYBIND.DAT        | Keybinds                         | Section    |          ✔️        |          ✗         |
| LOGFLTR.DAT        | Chat log filters?                | Block      |          ✔️        |          ✗         |
| MACRO.DAT          | Character-specific macros        | Section    |          ✔️        |          ✗         |
| MACROSYS.DAT       | System-wide macros               | Sections   |          ✔️        |          ✗         |
| UISAVE.DAT         | UI config                        | Block      |          ✔️        |          ✗         |

## Special Thanks

[EmperorArthur/FFXIV_Settings](https://github.com/EmperorArthur/FFXIV_Settings) was my jumping off point for research when developing this library.

## Contributing

Contributions are always welcomed. Please ensure code passes `cargo test` and `rustfmt` before making pull requests.
