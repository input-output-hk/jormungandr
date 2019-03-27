use std::path::Path;

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
