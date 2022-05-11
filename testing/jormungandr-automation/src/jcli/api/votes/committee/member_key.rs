use crate::jcli::command::votes::committee::MemberKeyCommand;
use assert_cmd::assert::OutputAssertExt;
use assert_fs::{assert::PathAssert, fixture::FileWriteStr, NamedTempFile};
use jortestkit::prelude::ProcessOutput;
use std::path::Path;

pub struct MemberKey {
    member_key_command: MemberKeyCommand,
}

impl MemberKey {
    pub fn new(member_key_command: MemberKeyCommand) -> Self {
        Self { member_key_command }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_public<S: Into<String>>(
        self,
        member_secret_key: S,
    ) -> Result<String, std::io::Error> {
        let input_file = NamedTempFile::new("member_key.tmp").unwrap();
        input_file.write_str(&member_secret_key.into()).unwrap();

        let output_file = NamedTempFile::new("output_key.tmp").unwrap();

        self.member_key_command
            .to_public(input_file.path(), output_file.path())
            .build()
            .assert()
            .success();
        output_file.assert(jortestkit::prelude::file_exists_and_not_empty());
        jortestkit::prelude::read_file(output_file.path())
    }

    pub fn generate<P: AsRef<Path>, S: Into<String>>(
        self,
        communication_key: P,
        crs: S,
        index: u32,
        threshold: u32,
        maybe_seed: Option<String>,
    ) -> String {
        self.member_key_command
            .generate(communication_key, crs, index, threshold, maybe_seed)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }
}
