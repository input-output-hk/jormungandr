use jormungandr_lib::crypto::hash::Hash;
use serde_yaml::Error as SerdeError;
use std::{collections::BTreeMap, process::Output};

pub trait ProcessOutput {
    fn as_lossy_string(&self) -> String;
    fn as_single_line(&self) -> String;
    fn as_hash(&self) -> Hash;
    fn as_multi_node_yaml(&self) -> Vec<BTreeMap<String, String>>;
    fn as_single_node_yaml(&self) -> BTreeMap<String, String>;
    fn try_as_single_node_yaml(&self) -> Result<BTreeMap<String, String>, SerdeError>;
    fn err_as_lossy_string(&self) -> String;
    fn err_as_single_line(&self) -> String;
}

impl ProcessOutput for Output {
    fn as_lossy_string(&self) -> String {
        let content = String::from_utf8_lossy(&self.stdout).into_owned();
        content
    }

    fn as_single_line(&self) -> String {
        let mut content = self.as_lossy_string();
        if content.ends_with("\n") {
            let len = content.len();
            content.truncate(len - 1);
        }
        content.trim().to_string()
    }

    fn as_hash(&self) -> Hash {
        let single_line = self.as_single_line();
        let result = Hash::from_hex(&single_line);
        assert!(result.is_ok(), "Cannot parse line {} as hash", single_line);
        result.unwrap()
    }

    fn err_as_lossy_string(&self) -> String {
        let content = String::from_utf8_lossy(&self.stderr).into_owned();
        content
    }

    fn err_as_single_line(&self) -> String {
        let mut content = self.err_as_lossy_string();
        if content.ends_with("\n") {
            let len = content.len();
            content.truncate(len - 1);
        }
        content
    }

    fn as_multi_node_yaml(&self) -> Vec<BTreeMap<String, String>> {
        let content = self.as_lossy_string();
        let deserialized_map: Vec<BTreeMap<String, String>> =
            serde_yaml::from_str(&content).unwrap();
        deserialized_map
    }

    fn as_single_node_yaml(&self) -> BTreeMap<String, String> {
        let content = self.as_lossy_string();
        let deserialized_map: BTreeMap<String, String> = serde_yaml::from_str(&content).unwrap();
        deserialized_map
    }

    fn try_as_single_node_yaml(&self) -> Result<BTreeMap<String, String>, SerdeError> {
        let content = self.as_lossy_string();
        serde_yaml::from_str(&content)
    }
}
