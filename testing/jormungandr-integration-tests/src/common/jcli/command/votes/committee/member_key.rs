use std::{path::Path, process::Command};
pub struct MemberKeyCommand {
    command: Command,
}

impl MemberKeyCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_public<P: AsRef<Path>, Q: AsRef<Path>>(
        mut self,
        private_key_file: P,
        output_file: Q,
    ) -> Self {
        self.command
            .arg("to-public")
            .arg("--input")
            .arg(private_key_file.as_ref())
            .arg(output_file.as_ref());
        self
    }

    pub fn generate<P: AsRef<Path>, S: Into<String>>(
        mut self,
        communication_key: P,
        crs: S,
        index: u32,
        threshold: u32,
        maybe_seed: Option<String>,
    ) -> Self {
        self.command
            .arg("generate")
            .arg("--keys")
            .arg(communication_key.as_ref())
            .arg("--crs")
            .arg(crs.into())
            .arg("--index")
            .arg(index.to_string())
            .arg("--threshold")
            .arg(threshold.to_string());

        if let Some(seed) = maybe_seed {
            self.command.arg("--seed").arg(seed);
        }

        self
    }

    pub fn build(self) -> Command {
        println!("{:?}", self.command);
        self.command
    }
}
