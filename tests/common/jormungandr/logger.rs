use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(Debug)]
pub struct JormungandrLogger {
    pub log_file_path: PathBuf,
}

impl JormungandrLogger {
    pub fn new(log_file_path: PathBuf) -> Self {
        JormungandrLogger { log_file_path }
    }

    pub fn get_lines_with_error(&self) -> Vec<String> {
        let lines = self.get_lines_from_log();
        lines
            .iter()
            .filter(|n| n.to_uppercase().contains("ERROR"))
            .cloned()
            .collect::<Vec<String>>()
    }

    fn get_lines_from_log(&self) -> Vec<String> {
        let file = File::open(self.log_file_path.clone()).unwrap();
        let mut data: Vec<String> = Vec::new();
        let reader = BufReader::new(file);

        for (_index, line) in reader.lines().enumerate() {
            data.push(line.unwrap());
        }
        data
    }
}
