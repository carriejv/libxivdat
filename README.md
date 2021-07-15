# libxivdat

[![Crates.io](https://img.shields.io/crates/v/libxivdat.svg)](https://crates.io/crates/libxivdat/)
[![Doc.rs](https://docs.rs/libxivdat/badge.svg)](https://docs.rs/crate/libxivdat/)
[![Apache-2.0](https://img.shields.io/github/license/carriejv/libxivdat)](https://github.com/carriejv/libxivdat/blob/master/LICENSE/)
[![Build Status](https://github.com/carriejv/libxivdat/workflows/CIBuild/badge.svg?branch=master)](https://github.com/carriejv/libxivdat/actions?query=workflow%3ACIBuild)

A library for working with Final Fantasy XIV .DAT files. These files store client-side game config including macros, hotkeys, and ui settings.

This is still a work in progress and support for all DAT files is incomplete. See [future plans](#future-plans) for more info.

## Minimum Supported Rust Version

This library has been tested against Rust >=1.52.0. Earlier versions may work, but use at your own risk.

CI builds are run against current stable and nightly at time of build.

## DATFile

`DATFile` provides a common, low-level method of working with binary DAT files. `DATFile` emulates Rust's std lib `File` but reads and writes only from the inner content block of .DAT files, automatically handling header updates, padding, and masking as necessary.

## High Level Modules

Higher-level support for specific file types is implemented on a type-by-type basis as optional features. See the [chart below](#dat-type-support) for more information and feature names.

High level modules allow working with DAT files at a resource level (ie, Macros or Gearsets) as opposed to working with raw byte streams from `DATFile`.

## DAT Data Content

Most DAT files (excluding those marked as "Unique" in the support table), share a common file structure consisting of a header, content block, and footer.

Internally, some DAT file content blocks use a variable-length data structure referred to as a `section` in this library. A section consists of a single UTF-8 char type tag, u16le size, and a null-terminated UTF-8 string. A single resource (ie, a macro) is then comprised of a repeating pattern of sections. A toolkit for working with sections is provided in the `section` submodule.

Other DAT files use fixed-size resource blocks, with each resource immediately following the last. These are referred to as "Block DATs" below.

## Plaintext DAT Files

Some DAT files (namely `COMMON.DAT`, `CONTROL0.DAT`, and `CONTROL1.DAT`) are actually just UTF-8 plaintext and do not share a common format with the binary DAT files.

Support for working with plaintext DATs may happen at some future point, but isn't currently a priority.

## Unique Binary DAT Files

Two binary file types (`ADDON.DAT` and `FFXIV_CHARA_XX.DAT` files) do not use the common shared structure of other DAT files. Support for these files is not currently planned.

## Future Plans

The goal of this library is to fully abstract the data structures of DAT files and create high-level, resource-based interfaces for each file type that operate on resources (ie, macros or gearsets) rather than raw byte streams. Each file has its own internal data types, so these layers will be implemented one at a time as optional features.

My focus with this libary is mainly on macros, gearsets, and ui config. High level support for other DAT types will liklely be a long time coming unless you build it yourself.

## DAT Type Support

| Symbol | Description     |
|--------|-----------------|
|   ✅   | Full support    |
|   🌀   | Partial support |
|   ❌   | No support      |

| File               | Contains                         | Type       | DATFile Read/Write | High Level Module |
|--------------------|----------------------------------|------------|--------------------|-------------------|
| ACQ.DAT            | Recent /tell history             | Section    |         ✅         |   🌀 - `section`  |
| ADDON.DAT          | ?                                | Unique     |         ❌         |         ❌        |
| COMMON.DAT         | Character configuration          | Plaintext  |         ❌         |         ❌        |
| CONTROL0.DAT       | Gamepad control config           | Plaintext  |         ❌         |         ❌        |
| CONTROL1.DAT       | Keyboard/mouse control config    | Plaintext  |         ❌         |         ❌        |
| FFXIV_CHARA_XX.DAT | Character appearance presets     | Unique     |         ❌         |         ❌        |
| GEARSET.DAT        | Gearsets                         | Block      |         ✅         |         ❌        |
| GS.DAT             | Gold Saucer config (Triad decks) | Block      |         ✅         |         ❌        |
| HOTBAR.DAT         | Hotbar layouts                   | Block      |         ✅         |         ❌        |
| ITEMFDR.DAT        | "Search for item" indexing?      | Block      |         ✅         |         ❌        |
| ITEMODR.DAT        | Item order in bags               | Block      |         ✅         |         ❌        |
| KEYBIND.DAT        | Keybinds                         | Section    |         ✅         |   🌀 - `section`  |
| LOGFLTR.DAT        | Chat log filters?                | Block      |         ✅         |         ❌        |
| MACRO.DAT          | Character-specific macros        | Section    |         ✅         |    ✅ - `macro`   |
| MACROSYS.DAT       | System-wide macros               | Section    |         ✅         |    ✅ - `macro`   |
| UISAVE.DAT         | UI config                        | Block      |         ✅         |         ❌        |

## Special Thanks

[EmperorArthur/FFXIV_Settings](https://github.com/EmperorArthur/FFXIV_Settings) was my jumping off point for research when developing this library.

## Contributing

Contributions are always welcomed. Please ensure code passes `cargo test --all-features`, `cargo clippy --all-features`, and `rustfmt -v --check **/*.rs` before making pull requests.
