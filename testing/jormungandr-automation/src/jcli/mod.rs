mod api;
mod command;
mod data;
mod services;

use super::jormungandr::JormungandrProcess;
use crate::testing::configuration;
use api::{Address, Certificate, Genesis, Key, Rest, Transaction, Votes};
pub use command::JCliCommand;
pub use data::{Witness, WitnessData, WitnessType};
use jormungandr_lib::crypto::hash::Hash;
pub use services::{
    CertificateBuilder, Error, FragmentCheck, FragmentSender, FragmentsCheck, TransactionBuilder,
};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Debug)]
pub struct JCli {
    exe: PathBuf,
}

impl Default for JCli {
    fn default() -> Self {
        Self::new(configuration::get_jcli_app())
    }
}

impl JCli {
    pub fn new(exe: PathBuf) -> Self {
        Self { exe }
    }

    pub fn path(&self) -> &Path {
        self.exe.as_path()
    }

    pub fn genesis(&self) -> Genesis {
        let command = Command::new(self.exe.clone());
        let jcli_command = JCliCommand::new(command);
        Genesis::new(jcli_command.genesis())
    }

    pub fn key(&self) -> Key {
        let command = Command::new(self.exe.clone());
        let jcli_command = JCliCommand::new(command);
        Key::new(jcli_command.key())
    }

    pub fn address(&self) -> Address {
        let command = Command::new(self.exe.clone());
        let jcli_command = JCliCommand::new(command);
        Address::new(jcli_command.address())
    }

    pub fn rest(&self) -> Rest {
        let command = Command::new(self.exe.clone());
        let jcli_command = JCliCommand::new(command);
        Rest::new(jcli_command.rest())
    }

    pub fn transaction(&self) -> Transaction {
        let command = Command::new(self.exe.clone());
        let jcli_command = JCliCommand::new(command);
        Transaction::new(jcli_command.transaction())
    }

    pub fn certificate(&self) -> Certificate {
        let command = Command::new(self.exe.clone());
        let jcli_command = JCliCommand::new(command);
        Certificate::new(jcli_command.certificate())
    }

    pub fn votes(&self) -> Votes {
        let command = Command::new(self.exe.clone());
        let jcli_command = JCliCommand::new(command);
        Votes::new(jcli_command.votes())
    }

    pub fn fragment_sender<'a>(&self, jormungandr: &'a JormungandrProcess) -> FragmentSender<'a> {
        FragmentSender::new(self.clone(), jormungandr)
    }

    pub fn transaction_builder(&self, genesis_hash: Hash) -> TransactionBuilder {
        TransactionBuilder::new(self.clone(), genesis_hash)
    }

    pub fn certificate_builder(&self) -> CertificateBuilder {
        CertificateBuilder::new(self.clone())
    }

    pub fn fragments_checker<'a>(&self, jormungandr: &'a JormungandrProcess) -> FragmentsCheck<'a> {
        FragmentsCheck::new(self.clone(), jormungandr)
    }
}
