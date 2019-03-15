use error::ErrorKind;

use std::fs;
use std::fs::{File, OpenOptions};
use std::io::BufReader;
use std::io::BufWriter;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;

/// Reads file and returns contents or error as Result
pub fn read_file(path: &String) -> Result<Vec<String>, ErrorKind> {
    let file = match OpenOptions::new().read(true).open(path) {
        Err(_) => {
            return Err(ErrorKind::FileOpenError);
        },
        Ok(f) => f,
    };

    let mut reader = BufReader::new(file);
    let mut lines = Vec::new();
    let mut line = String::new();

    loop {
        match reader.read_line(&mut line) {
            Err(e) => return Err(ErrorKind::FileReadError),
            Ok(len) => if len == 0 {
                break;
            },
        }
        
        // line = line.replace("\n", "");
        lines.push(line.clone());
        line.clear();
    }

    Ok(lines)
}

pub fn write_file(path: &String, lines: &Vec<String>) -> Result<(), ErrorKind> {
    // create/open file
    let f = match File::create(&Path::new(path)) {
        Err(e) => return Err(ErrorKind::FileCreationError),
        Ok(f) => f,
    };
    let mut writer = BufWriter::new(&f);
    // write lines
    for n in 0..lines.len() {
        match write!(writer, "{}\n", lines[n]) {
            Err(e)  => return Err(ErrorKind::FileWriteError),
            Ok(_)   => {},
        }
    }
    match writer.flush() {
        Err(e)  => return Err(ErrorKind::FileWriteError),
        Ok(_)   => {},
    }
    Ok(())
}
