use std::fs::metadata;

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
pub fn assert_file_exists(file_name: &str) {
    let metadata = metadata(&file_name).unwrap();
    assert!(
        metadata.is_file(),
        "file '{}' does not exists, while it should",
        &file_name
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
pub fn assert_file_not_empty(file_name: &str) {
    let metadata = metadata(&file_name).unwrap();
    assert!(
        metadata.len() > 0,
        "file '{}' is empty (len = {}), while it shouldn't",
        &file_name,
        metadata.len()
    );
}
