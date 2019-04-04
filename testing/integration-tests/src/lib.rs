pub mod file_assert;

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
pub fn assert_file_exists_and_not_empty(file_name: &str) {
    file_assert::assert_file_exists(&file_name);
    file_assert::assert_file_not_empty(&file_name);
}
