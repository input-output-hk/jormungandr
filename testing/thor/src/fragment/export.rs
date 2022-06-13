#![allow(dead_code)]

use crate::wallet::Wallet;
use chain_core::{packer::Codec, property::DeserializeFromSlice};
use chain_impl_mockchain::fragment::{Fragment, FragmentId};
use jormungandr_automation::jormungandr::FragmentNode;
use jormungandr_lib::interfaces::Address;
use std::{fs, io::Write, path::PathBuf};
use thiserror::Error;
use time::OffsetDateTime;

#[derive(Debug, Error)]
pub enum FragmentExporterError {
    #[error("cannot create dump folder {0}")]
    CannotCreateDumpFolder(PathBuf),
    #[error("cannot create dump file {0}")]
    CannotCreateDumpFile(PathBuf),
    #[error("cannot write fragment bin to file {0}")]
    CannotWriteFragmentToDumpFile(PathBuf),
    #[error("io error")]
    IoError(#[from] std::io::Error),
}

pub struct FragmentExporter {
    dump_folder: PathBuf,
}

impl FragmentExporter {
    pub fn new(dump_folder: PathBuf) -> Result<Self, FragmentExporterError> {
        fs::create_dir_all(dump_folder.clone())
            .map_err(|_| FragmentExporterError::CannotCreateDumpFolder(dump_folder.clone()))?;

        Ok(Self { dump_folder })
    }

    pub fn read(&self) -> Result<Vec<Fragment>, FragmentExporterError> {
        self.read_as_bytes()?
            .iter()
            .map(|bytes| {
                Ok(Fragment::deserialize_from_slice(&mut Codec::new(bytes.as_ref())).unwrap())
            })
            .collect()
    }

    pub fn read_as_bytes(&self) -> Result<Vec<Vec<u8>>, FragmentExporterError> {
        let mut entries = fs::read_dir(&self.dump_folder)?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, std::io::Error>>()?;
        entries.sort();
        // the order is platform dependant, let's sort again in time order
        entries
            .into_iter()
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .ends_with(".fragment")
            })
            .map(|path| {
                let content = jortestkit::prelude::read_file(path).unwrap();
                let bytes = hex::decode(content.trim()).unwrap();
                Ok(bytes)
            })
            .collect()
    }

    pub fn dump_to_file(
        &self,
        fragment: &Fragment,
        sender: &Wallet,
        via: &dyn FragmentNode,
    ) -> Result<(), FragmentExporterError> {
        let file_name = self.generate_file_name(fragment, sender, via);
        let file_path = self.dump_folder.join(file_name);

        let mut file = fs::File::create(&file_path)
            .map_err(|_| FragmentExporterError::CannotCreateDumpFile(file_path))?;

        file.write_all(self.format_fragment(fragment).as_bytes())
            .map_err(|_| {
                FragmentExporterError::CannotWriteFragmentToDumpFile(self.dump_folder.clone())
            })?;

        Ok(())
    }

    pub fn dump_to_file_no_sender(
        &self,
        fragment: &Fragment,
        via: &dyn FragmentNode,
    ) -> Result<(), FragmentExporterError> {
        let file_name = self.generate_file_name_without_sender(fragment, via);
        let file_path = self.dump_folder.join(file_name);
        let mut file = fs::File::create(&file_path)
            .map_err(|_| FragmentExporterError::CannotCreateDumpFile(file_path))?;

        file.write_all(self.format_fragment(fragment).as_bytes())
            .map_err(|_| {
                FragmentExporterError::CannotWriteFragmentToDumpFile(self.dump_folder.clone())
            })?;

        Ok(())
    }

    fn generate_file_name_without_sender(
        &self,
        fragment: &Fragment,
        via: &dyn FragmentNode,
    ) -> String {
        let now = OffsetDateTime::now_utc();
        let alias = {
            if via.alias().is_empty() {
                "jormungandr".to_string()
            } else {
                via.alias()
            }
        };

        format!(
            "{}_{}_to_{}.fragment",
            now.format(time::macros::format_description!(
                "[year]-[month]-[day]_[hour]_[minute]_[second]_[subsecond]"
            ))
            .unwrap(),
            self.format_id(fragment.hash()),
            alias
        )
    }

    fn generate_file_name(
        &self,
        fragment: &Fragment,
        sender: &Wallet,
        via: &dyn FragmentNode,
    ) -> String {
        let now = OffsetDateTime::now_utc();
        let alias = {
            if via.alias().is_empty() {
                "jormungandr".to_string()
            } else {
                via.alias()
            }
        };

        format!(
            "{}_{}_from_{}_to_{}.fragment",
            now.format(time::macros::format_description!(
                "[year]-[month]-[day]_[hour]_[minute]_[second]_[subsecond]"
            ))
            .unwrap(),
            self.format_id(fragment.hash()),
            self.format_address(sender.address()),
            alias
        )
    }

    fn format_fragment(&self, fragment: &Fragment) -> String {
        use chain_core::property::Serialize;

        let bytes = fragment.serialize_as_vec().unwrap();
        hex::encode(&bytes)
    }

    fn format_address(&self, address: Address) -> String {
        self.format_hash(address.to_string())
    }

    fn format_id(&self, id: FragmentId) -> String {
        self.format_hash(id.to_string())
    }

    fn format_hash(&self, hash: String) -> String {
        hash[..6].to_owned()
    }
}
