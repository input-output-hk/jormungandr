use assert_fs::assert::IntoPathPredicate;
use predicates::prelude::*;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn file_exists_and_not_empty() -> impl Predicate<Path> {
    predicate::path::exists().and([].into_path().not())
}

pub fn file_binary_content_is_same_as(expected: impl Into<PathBuf>) -> impl Predicate<Path> {
    predicate::path::eq_file(expected)
}

pub fn file_text_content_is_same_as(expected: impl AsRef<Path>) -> impl Predicate<Path> {
    // predicate::path::eq_file(expected).utf8().unwrap() does not do diffs:
    // https://github.com/assert-rs/predicates-rs/issues/65
    let mut expected_content = String::new();
    let mut file = File::open(expected).unwrap();
    file.read_to_string(&mut expected_content).unwrap();
    expected_content.into_path()
}
