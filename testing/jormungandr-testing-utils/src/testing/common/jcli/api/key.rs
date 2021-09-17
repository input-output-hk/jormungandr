use crate::testing::common::jcli::command::KeyCommand;
use assert_cmd::assert::OutputAssertExt;
use assert_fs::{fixture::FileWriteStr, NamedTempFile};
use jortestkit::prelude::ProcessOutput;
use std::path::Path;
const DEFAULT_KEY_TYPE: &str = "Ed25519Extended";

pub struct Key {
    key_command: KeyCommand,
}

impl Key {
    pub fn new(key_command: KeyCommand) -> Self {
        Self { key_command }
    }

    pub fn generate_default(self) -> String {
        self.generate(DEFAULT_KEY_TYPE)
    }

    pub fn generate<S: Into<String>>(self, key_type: S) -> String {
        self.key_command
            .generate()
            .key_type(key_type.into())
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn generate_expect_fail<S: Into<String>>(self, key_type: S, expected_msg_path: &str) {
        self.key_command
            .generate()
            .key_type(key_type.into())
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg_path));
    }

    pub fn generate_with_seed<S: Into<String>>(self, key_type: S, seed: S) -> String {
        self.key_command
            .generate()
            .key_type(key_type.into())
            .seed(seed.into())
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn generate_with_seed_expect_fail<S: Into<String>>(
        self,
        key_type: S,
        seed: S,
        expected_msg_path: &str,
    ) {
        self.key_command
            .generate()
            .key_type(key_type.into())
            .seed(seed.into())
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg_path));
    }

    pub fn convert_to_public_string<S: Into<String>>(self, private_key: S) -> String {
        let input_file = NamedTempFile::new("key_to_public.input").unwrap();
        input_file.write_str(&private_key.into()).unwrap();

        self.key_command
            .to_public()
            .input(input_file.path())
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn convert_to_public_string_expect_fail<S: Into<String>>(
        self,
        private_key: S,
        expected_msg_path: &str,
    ) {
        let input_file = NamedTempFile::new("key_to_public.input").unwrap();
        input_file.write_str(&private_key.into()).unwrap();

        self.key_command
            .to_public()
            .input(input_file.path())
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg_path));
    }

    pub fn dump_bytes_to_file<S: Into<String>, P: AsRef<Path>>(self, private_key: S, output: P) {
        let input = NamedTempFile::new("key_to_bytes.input").unwrap();
        input.write_str(&private_key.into()).unwrap();

        self.convert_to_bytes_file(input.path(), output.as_ref())
    }

    pub fn convert_to_bytes_file<P: AsRef<Path>, Q: AsRef<Path>>(self, input: P, output: Q) {
        self.key_command
            .to_bytes()
            .output(output)
            .input(input)
            .build()
            .assert()
            .success();
    }

    pub fn convert_to_bytes_file_expect_fail<P: AsRef<Path>, Q: AsRef<Path>>(
        self,
        input: P,
        output: Q,
        expected_msg_path: &str,
    ) {
        self.key_command
            .to_bytes()
            .output(output)
            .input(input)
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg_path));
    }

    pub fn convert_from_bytes_string<P: AsRef<Path>, S: Into<String>>(
        self,
        key_type: S,
        input: P,
    ) -> String {
        self.key_command
            .from_bytes()
            .key_type(key_type)
            .input(input)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn convert_from_bytes_string_expect_fail<P: AsRef<Path>, S: Into<String>>(
        self,
        key_type: S,
        input: P,
        expected_msg_path: &str,
    ) {
        self.key_command
            .from_bytes()
            .key_type(key_type)
            .input(input)
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg_path));
    }
}
