use crate::jcli_lib::utils::io;
use std::{io::Write, path::PathBuf};
use structopt::StructOpt;

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

impl From<PathBuf> for OutputFile {
    fn from(output: PathBuf) -> Self {
        Self {
            output: Some(output),
        }
    }
}

impl OutputFile {
    pub fn open(&self) -> Result<impl Write, Error> {
        io::open_file_write(&self.output).map_err(|cause| Error::CannotOpen {
            cause,
            path: self.output.clone().unwrap_or_default(),
        })
    }
}
