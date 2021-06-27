use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Stdout, Write};

pub mod dat_error;
pub mod dat_file;
pub mod dat_type;

fn main() {
    read(&std::env::args().collect::<Vec<String>>()[1])
}

fn read(path: &str) {
    let content_bytes = dat_file::read_content(path).unwrap();
    let mut out = std::io::stdout();
    out.write_all(&content_bytes).unwrap();
    out.flush().unwrap();
}
