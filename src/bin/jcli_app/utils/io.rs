use std::io::{stdin, stdout, BufRead, BufReader, Error, Write};
use std::path::Path;

/// open the given file path as a writable stream, or stdout if no path
/// provided
pub fn open_file_write<P: AsRef<Path>>(path: &Option<P>) -> Result<impl Write, Error> {
    match path {
        Some(path) => {
            let writer = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .read(false)
                .append(false)
                .truncate(true)
                .open(path)?;
            Ok(Box::new(writer) as Box<Write>)
        }
        None => Ok(Box::new(stdout()) as Box<Write>),
    }
}

/// open the given file path as a readable stream, or stdin if no path
/// provided
pub fn open_file_read<P: AsRef<Path>>(path: &Option<P>) -> Result<impl BufRead, Error> {
    match path {
        Some(path) => {
            let reader = std::fs::OpenOptions::new()
                .create(false)
                .write(false)
                .read(true)
                .append(false)
                .open(path)?;
            Ok(Box::new(BufReader::new(reader)) as Box<dyn BufRead>)
        }
        None => Ok(Box::new(BufReader::new(stdin())) as Box<dyn BufRead>),
    }
}
