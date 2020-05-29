#![allow(dead_code)]

use std::fs;
use std::path::Path;
use std::path::PathBuf;

pub fn find_file<P: AsRef<Path>>(root: P, part_of_name: &str) -> Option<PathBuf> {
    for entry in fs::read_dir(root).expect("cannot read root directory") {
        let entry = entry.unwrap();
        if entry.file_name().to_str().unwrap().contains(part_of_name) {
            return Some(entry.path());
        }
    }
    None
}

pub fn read_file(path: impl AsRef<Path>) -> String {
    let contents = fs::read_to_string(path).expect("cannot read file");
    trim_new_line_at_end(contents)
}

fn trim_new_line_at_end(mut content: String) -> String {
    if content.ends_with('\n') {
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
