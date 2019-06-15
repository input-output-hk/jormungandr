use std::io::{stdin, stdout, BufRead, BufReader, Error, Write};
use std::path::Path;
use std::path::PathBuf;

extern crate mktemp;

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

/// get path to unique file in temp folder
pub fn get_path_in_temp<T: Into<String>>(file_path: T) -> Result<PathBuf, std::io::Error> {
    let mut path = get_temp_folder().unwrap();
    path.push(file_path.into());
    Ok(path)
}

/// get os temp folder
pub fn get_temp_folder() -> Result<PathBuf, std::io::Error> {
    let temp_dir = mktemp::Temp::new_dir().unwrap();
    let path = temp_dir.to_path_buf();
    temp_dir.release();
    Ok(path)
}

pub fn path_to_path_buf<P: AsRef<Path>>(path: &Option<P>) -> PathBuf {
    path.as_ref()
        .map(|path| path.as_ref().to_path_buf())
        .unwrap_or_default()
}

pub fn read_line<P: AsRef<Path>>(path: &Option<P>) -> Result<String, std::io::Error> {
    let mut line = String::new();
    open_file_read(path)?.read_line(&mut line)?;
    Ok(line.trim_end().to_string())
}
