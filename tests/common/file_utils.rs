#![allow(dead_code)]

extern crate mktemp;

use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

/// Gets path in temp directory (does not create it)
///
/// # Arguments
///
/// * `file_path` - A string slice that holds the path
/// that will be glued to temp directory path
///
/// # Example
///
/// use file_utils::get_path_in_temp;
/// let path_in_temp = "test.txt";
/// get_path_in_temp(&path_in_temp);
///
pub fn get_path_in_temp(file_path: &str) -> PathBuf {
    let mut path = get_temp_folder();
    path.push(&file_path);
    path
}

pub fn create_empty_file_in_temp(file_name: &str) -> PathBuf {
    let path = create_file_in_temp(&file_name, "");
    path
}

pub fn get_temp_folder() -> PathBuf {
    let temp_dir = mktemp::Temp::new_dir().unwrap();
    let path = temp_dir.to_path_buf();
    temp_dir.release();
    path
}

/// Creates file in temporary folder
pub fn create_file_in_temp(file_name: &str, content: &str) -> PathBuf {
    let path = get_path_in_temp(&file_name);
    let mut file = File::create(&path).unwrap();
    file.write_all(content.as_bytes())
        .expect(&format!("cannot write to file {:?}", path));
    path
}

/// Creates file with content
pub fn create_file_with_content(path: &PathBuf, content: &str) -> () {
    let mut file = File::create(&path).unwrap();
    file.write_all(content.as_bytes())
        .expect(&format!("cannot write to file {:?}", path));
}

pub fn read_file(path: &PathBuf) -> String {
    let contents = fs::read_to_string(path).expect("cannot read file");
    contents
}
