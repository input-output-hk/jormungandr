use bawawa::{Command, Error, Process, Program, StandardOutput};
use futures::stream::Stream;
use tokio_codec::LinesCodec;

pub struct Openssl {
    program: Program,
}

impl Openssl {
    pub fn new() -> Result<Self, Error> {
        Ok(Openssl {
            program: Program::new("openssl".to_owned())?,
        })
    }

    pub fn version(&self) -> Result<String, Error> {
        let mut openssl = Command::new(self.program.clone());
        openssl.argument("version");
        self.echo_stdout(openssl)
    }

    fn echo_stdout(&self, cmd: Command) -> Result<String, Error> {
        let mut captured = Process::spawn(cmd)?
            .capture_stdout(LinesCodec::new())
            .wait();
        captured.next().unwrap()
    }
}
