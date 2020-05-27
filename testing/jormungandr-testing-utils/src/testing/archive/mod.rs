extern crate flate2;
extern crate tar;
extern crate zip;

use flate2::read::GzDecoder;
use std::fs::File;
use std::io;
use std::path::Path;
use tar::Archive;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DecompressError {
    #[error("could not open input file")]
    CannotOpenArchiveFile,
    #[error("could not write output file")]
    CannotWriteOutputFile(#[from] io::Error),
    #[error("internal unpack error")]
    IntenralZipError(#[from] zip::result::ZipError),
    #[error("internal unpack error")]
    UnpackError,
    #[error("unsupported format")]
    UnsupportedFormat,
}

pub fn decompress(input: &Path, output: &Path) -> Result<(), DecompressError> {
    let path = input
        .as_os_str()
        .to_str()
        .expect("cannot convert input path to os str");
    if path.ends_with(".zip") {
        let file = File::open(&path).map_err(|_| DecompressError::CannotOpenArchiveFile)?;
        let mut archive = zip::ZipArchive::new(file)?;
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|_| DecompressError::UnpackError)?;
            let outpath = output.join(file.sanitized_name());
            let mut outfile =
                File::create(&outpath).map_err(DecompressError::CannotWriteOutputFile)?;
            io::copy(&mut file, &mut outfile)?;
        }
        return Ok(());
    } else if path.ends_with(".tar.gz") {
        let tar_gz = File::open(path).map_err(|_| DecompressError::CannotOpenArchiveFile)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        archive
            .unpack(output)
            .map_err(|_| DecompressError::UnpackError)?;
        return Ok(());
    }
    Err(DecompressError::UnsupportedFormat)
}
