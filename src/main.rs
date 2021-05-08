use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::io::{Read, Seek, Stdout, Write};

pub mod dat_error;
pub mod dat_file;
pub mod dat_type;

fn main() {
    read(&std::env::args().collect::<Vec<String>>()[1])
}

fn read(path: &str) {
    let mut dat =
        dat_file::DATFile::open_options(path, std::fs::OpenOptions::new().read(true).write(true))
            .unwrap();

    let mut content_bytes = vec![0u8; dat.content_size().try_into().unwrap()];
    dat.read(&mut content_bytes).unwrap();

    let mut out = std::io::stdout();
    out.write_all(&content_bytes).unwrap();
    out.flush().unwrap()
}
