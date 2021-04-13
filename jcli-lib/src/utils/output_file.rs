use crate::utils::io;

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

#[derive(StructOpt, Debug)]
pub struct OutputFile {
    /// output the key to the given file or to stdout if not provided
    #[structopt(name = "OUTPUT_FILE")]
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
