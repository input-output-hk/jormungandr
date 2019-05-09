use std::path::Path;
use std::path::PathBuf;

extern crate mktemp;

/// open the given file path as a writable stream, or stdout if no path
/// provided
pub fn open_file_write<P: AsRef<Path>>(path: &Option<P>) -> Box<dyn std::io::Write> {
    if let Some(path) = path {
        Box::new(
            std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .read(false)
                .append(false)
                .open(path)
                .unwrap(),
        )
    } else {
        Box::new(std::io::stdout())
    }
}

/// open the given file path as a readable stream, or stdin if no path
/// provided
pub fn open_file_read<P: AsRef<Path>>(path: &Option<P>) -> Box<dyn std::io::BufRead> {
    if let Some(path) = path {
        Box::new(std::io::BufReader::new(
            std::fs::OpenOptions::new()
                .create(false)
                .write(false)
                .read(true)
                .append(false)
                .open(path)
                .unwrap(),
        ))
    } else {
        Box::new(std::io::BufReader::new(std::io::stdin()))
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
