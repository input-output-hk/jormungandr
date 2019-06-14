/// get path to unique file in temp folder
pub fn get_path_in_temp<T: Into<String>>(file_path: T) -> Result<PathBuf, std::io::Error> {
    let mut path = get_temp_folder().unwrap();
    path.push(file_path.into());
    Ok(path)
}

/// get os temp folder
pub fn get_temp_folder() -> Result<PathBuf, std::io::Error> {
    let temp_dir = mktemp::Temp::new_dir().unwrap();
    let path = temp_dir.to_path_buf();
    temp_dir.release();
    Ok(path)
}
