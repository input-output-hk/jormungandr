use std::fs::metadata;
use std::path::Path;

/// Assert input file exists, is actually a file and has more than 0 bytes
///
/// # Arguments
///
/// * `file_name` - A string slice that holds the name of the file
///
/// # Example
///
/// use integration_tests::assert_file_exists_and_not_empty;
/// let file_name = "./test.txt";
/// assert_file_exists_and_not_empty(&file_name);
///
pub fn assert_file_exists_and_not_empty<P: AsRef<Path>>(file_name: P) {
    assert_file_exists(&file_name);
    assert_file_not_empty(&file_name);
}

/// Assert input file exists and is actually a file
///
/// # Arguments
///
/// * `file_name` - A string slice that holds the name of the file
///
/// # Example
///
/// use integration_tests::file_assert::assert_file_exists;
/// let file_name = "./test.txt";
/// assert_file_exists(&file_name);
///
pub fn assert_file_exists<P: AsRef<Path>>(file_name: P) {
    assert!(
        file_name.as_ref().exists(),
        "file '{:?}' does not exist",
        file_name.as_ref()
    );
}

/// Assert input file has more than 0 bytes
///
/// # Arguments
///
/// * `file_name` - A string slice that holds the name of the file
///
/// # Example
///
/// use integration_tests::file_assert::assert_file_not_empty;
/// let file_name = "./test.txt";
/// assert_file_not_empty(&file_name);
///
pub fn assert_file_not_empty<P: AsRef<Path>>(file_name: P) {
    let metadata = metadata(file_name.as_ref())
        .expect(&format!("file '{:?}' does not exist", file_name.as_ref()));

    assert!(
        metadata.len() > 0,
        "file '{:?}' is empty",
        file_name.as_ref()
    );
}
