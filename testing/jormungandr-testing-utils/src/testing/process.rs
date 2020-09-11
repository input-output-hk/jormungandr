use jormungandr_lib::crypto::hash::Hash;
use jortestkit::prelude::ProcessOutput as _;
use std::process::Output;

pub trait ProcessOutput {
    fn as_hash(&self) -> Hash;
}

impl ProcessOutput for Output {
    fn as_hash(&self) -> Hash {
        let single_line = self.as_single_line();
        let result = Hash::from_hex(&single_line);
        assert!(result.is_ok(), "Cannot parse line {} as hash", single_line);
        result.unwrap()
    }
}
