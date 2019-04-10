use mktemp::Temp;
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
    let temp_dir = Temp::new_dir().unwrap();
    let mut path = temp_dir.to_path_buf();
    path.push(&file_path);
    temp_dir.release();
    path
}
