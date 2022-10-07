use jormungandr_lib::{crypto::hash::Hash, interfaces::SettingsDto};
use std::str::FromStr;

pub trait SettingsDtoExtension {
    fn genesis_block_hash(&self) -> Hash;
}

impl SettingsDtoExtension for SettingsDto {
    fn genesis_block_hash(&self) -> Hash {
        Hash::from_str(&self.block0_hash).unwrap()
    }
}
