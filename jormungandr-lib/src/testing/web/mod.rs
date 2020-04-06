extern crate reqwest;
use std::fs::File;
use std::io;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WebError {
    #[error("could not download file")]
    CannotDownloadFile(#[from] reqwest::Error),
    #[error("could not save output to file")]
    CannotCreateOutputFile,
    #[error("could not send reqeuest")]
    IOError(#[from] io::Error),
}

pub fn download_file(link: String, output: &Path) -> Result<(), WebError> {
    let mut resp = reqwest::blocking::get(&link).map_err(|e| WebError::CannotDownloadFile(e))?;
    let mut out = File::create(output.as_os_str()).map_err(|_| WebError::CannotCreateOutputFile)?;
    io::copy(&mut resp, &mut out)?;
    Ok(())
}
