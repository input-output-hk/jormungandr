use crate::jcli::command::votes::committee::CommunicationKeyCommand;
use assert_cmd::assert::OutputAssertExt;
use assert_fs::{assert::PathAssert, fixture::FileWriteStr, NamedTempFile};
use jortestkit::prelude::ProcessOutput;
pub struct CommunicationKey {
    communication_key_command: CommunicationKeyCommand,
}

impl CommunicationKey {
    pub fn new(communication_key_command: CommunicationKeyCommand) -> Self {
        Self {
            communication_key_command,
        }
    }

    pub fn generate(self) -> String {
        self.communication_key_command
            .generate()
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_public(self, input: String) -> Result<String, std::io::Error> {
        let input_file = NamedTempFile::new("input.tmp").unwrap();
        input_file.write_str(&input).unwrap();
        let output_file = NamedTempFile::new("output.tmp").unwrap();
        self.communication_key_command
            .to_public(input_file.path(), output_file.path())
            .build()
            .assert()
            .success();
        output_file.assert(jortestkit::prelude::file_exists_and_not_empty());
        jortestkit::prelude::read_file(output_file.path())
    }
}
