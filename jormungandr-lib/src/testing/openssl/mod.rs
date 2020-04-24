use bawawa::{Command, Error, Process, Program, StandardError, StandardOutput};
use futures::stream::Stream;
use tokio_codec::LinesCodec;

use std::path::PathBuf;

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
        let captured = Process::spawn(cmd.clone())?
            .capture_stdout(LinesCodec::new())
            .capture_stderr(LinesCodec::new())
            .wait();
        println!("{}", cmd);

        let lines: Vec<String> = captured
            .into_iter()
            .map(|r| r.unwrap_or_else(|_| "".to_owned()))
            .collect();
        Ok(format!("{}", lines.join("\n")))
    }

    pub fn genrsa(&self, length: u32, out_file: &PathBuf) -> Result<String, Error> {
        let mut openssl = Command::new(self.program.clone());
        openssl.arguments(&["genrsa", "-out", path_to_str(out_file), &length.to_string()]);
        self.echo_stdout(openssl)
    }

    pub fn pkcs8(&self, in_file: &PathBuf, out_file: &PathBuf) -> Result<String, Error> {
        let mut openssl = Command::new(self.program.clone());
        openssl.arguments(&[
            "pkcs8",
            "-topk8",
            "-inform",
            "PEM",
            "-outform",
            "PEM",
            "-in",
            in_file.as_os_str().to_str().unwrap(),
            "-out",
            path_to_str(out_file),
            "-nocrypt",
        ]);
        self.echo_stdout(openssl)
    }

    pub fn req(&self, prv_key: &PathBuf, out_cert: &PathBuf) -> Result<String, Error> {
        let mut openssl = Command::new(self.program.clone());
        openssl.arguments(&[
            "req",
            "-new",
            "-key",
            path_to_str(prv_key),
            "-out",
            path_to_str(out_cert),
            "-batch",
        ]);
        self.echo_stdout(openssl)
    }

    pub fn x509(
        &self,
        prv_key: &PathBuf,
        in_cert: &PathBuf,
        out_cert: &PathBuf,
    ) -> Result<String, Error> {
        let mut openssl = Command::new(self.program.clone());
        openssl.arguments(&[
            "x509",
            "-req",
            "-days",
            &3650.to_string(),
            "-in",
            path_to_str(in_cert),
            "-signkey",
            path_to_str(prv_key),
            "-out",
            path_to_str(out_cert),
        ]);
        self.echo_stdout(openssl)
    }
}

fn path_to_str(path: &PathBuf) -> &str {
    path.as_os_str().to_str().unwrap()
}
