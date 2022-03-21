use crate::jcli_lib::{
    certificate::{write_cert, Error},
    utils::key_parser::parse_pub_key,
};
use chain_crypto::{Ed25519, PublicKey};
use chain_impl_mockchain::{
    certificate::{Certificate, EvmMapping},
    evm::Address,
    transaction::UnspecifiedAccountIdentifier,
};
use jormungandr_lib::interfaces::Certificate as CertificateType;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
pub struct EvmMapCmd {
    /// jormungandr account id
    #[structopt(name = "ACCOUNT_KEY", parse(try_from_str = parse_pub_key))]
    account_id: PublicKey<Ed25519>,
    /// hex encoded H160 address
    evm_address: Address,
    /// write the output to the given file or print it to the standard output if not defined
    #[structopt(short = "o", long = "output")]
    output: Option<PathBuf>,
}

impl EvmMapCmd {
    pub fn exec(self) -> Result<(), Error> {
        let content = EvmMapping {
            account_id: UnspecifiedAccountIdentifier::from_single_account(self.account_id.into()),
            evm_address: self.evm_address,
        };
        let cert = Certificate::EvmMapping(content);
        write_cert(self.output.as_deref(), CertificateType(cert))
    }
}
