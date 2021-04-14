use crate::utils::io;

#[cfg(feature = "structopt")]
use structopt::StructOpt;

use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid output file path '{path}'")]
    CannotOpen {
        #[source]
        cause: std::io::Error,
        path: PathBuf,
    },
}

#[derive(Debug)]
#[cfg_attr(feature = "structopt", derive(StructOpt))]
pub struct OutputFile {
    /// output the key to the given file or to stdout if not provided
    #[cfg_attr(feature = "structopt", structopt(name = "OUTPUT_FILE"))]
    output: Option<PathBuf>,
}

impl OutputFile {
    pub fn open(&self) -> Result<impl Write, Error> {
        io::open_file_write(&self.output).map_err(|cause| Error::CannotOpen {
            cause,
            path: self.output.clone().unwrap_or_default(),
        })
    }
}
