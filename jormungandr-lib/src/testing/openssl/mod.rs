use bawawa::{Command, Error, Process, Program, StandardOutput};
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
        let mut captured = Process::spawn(cmd)?
            .capture_stdout(LinesCodec::new())
            .wait();
        captured.next().unwrap()
    }

    fn run(&self, cmd: Command) -> Result<(), Error> {
        let _captured = Process::spawn(cmd)?
            .capture_stdout(LinesCodec::new())
            .wait();
        Ok(())
    }
   
    pub fn genrsa(&self, length: u32, out_file: &PathBuf) -> Result<(), Error> {
        let mut openssl = Command::new(self.program.clone());
        openssl.arguments(&["-out",path_to_str(out_file),&length.to_string()]);
        self.run(openssl) 
    }

    pub fn pkcs8(&self, in_file: &PathBuf, out_file: &PathBuf) -> Result<(), Error> {
        let mut openssl = Command::new(self.program.clone());
        openssl.arguments(&["pkcs8","-topk8","-inform","PEM","-outform","PEM",
          "-in",in_file.as_os_str().to_str().unwrap(),"-out",path_to_str(out_file),"-nocrypt"]);
        self.run(openssl) 
    }

    pub fn req(&self,prv_key: &PathBuf, out_cert: &PathBuf) -> Result<(), Error> {
        let mut openssl = Command::new(self.program.clone());
        openssl.arguments(&["req","-new","-key",path_to_str(prv_key),"-out",path_to_str(out_cert)]);
        self.run(openssl)

    }

    pub fn x509(&self, prv_key: &PathBuf, in_cert: &PathBuf,  out_cert: &PathBuf) -> Result<(), Error> {
        let mut openssl = Command::new(self.program.clone());
        openssl.arguments(&["x509","-req","-days",&3650.to_string(),"-in",path_to_str(in_cert),"-signkey",path_to_str(prv_key),"-out",path_to_str(out_cert)]);
        self.run(openssl)
    }

}

fn path_to_str(path: &PathBuf) -> &str {
    path.as_os_str().to_str().unwrap()
}
