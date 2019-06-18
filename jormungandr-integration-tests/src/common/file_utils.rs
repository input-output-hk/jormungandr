#![allow(dead_code)]

extern crate mktemp;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

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
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(content.as_bytes())
        .expect(&format!("cannot write to file {:?}", path));
    path
}

/// Creates file with content
pub fn create_file_with_content(path: &PathBuf, content: &str) -> () {
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(content.as_bytes())
        .expect(&format!("cannot write to file {:?}", path));
}

pub fn read_file<P: AsRef<Path>>(path: &P) -> String {
    let contents = fs::read_to_string(path).expect("cannot read file");
    trim_new_line_at_end(contents)
}

fn trim_new_line_at_end(mut content: String) -> String {
    if content.ends_with("\n") {
        let len = content.len();
        content.truncate(len - 1);
    }
    content
}

pub fn make_readonly(path: &PathBuf) {
    if !path.exists() {
        std::fs::File::create(&path).unwrap();
    }
    let mut perms = fs::metadata(path.as_os_str()).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(path.as_os_str(), perms).expect("cannot set permissions");
}
