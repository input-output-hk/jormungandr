use crate::jcli_lib::{
    certificate::{write_cert, Error},
    utils::io,
};
use chain_impl_mockchain::{
    certificate::{Certificate, VotePlan},
    vote::PayloadType,
};
use jormungandr_lib::interfaces::VotePlanDef;
use serde::Deserialize;
use std::path::PathBuf;
use structopt::StructOpt;

/// create a vote plan certificate
///
/// the vote plan configuration data needs to be provided
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct VotePlanRegistration {
    /// the file containing the vote plan configuration (YAML). If no file
    /// provided, it will be read from the standard input
    pub input: Option<PathBuf>,

    /// write the output to the given file or print it to the standard output if not defined
    #[structopt(long = "output")]
    pub output: Option<PathBuf>,
}

#[derive(Deserialize)]
struct VotePlanConfiguration(#[serde(with = "VotePlanDef")] VotePlan);

fn validate_voteplan(voteplan: &VotePlan) -> Result<(), Error> {
    // if voteplan is private committee member keys should be filled
    match voteplan.payload_type() {
        PayloadType::Public => {}
        PayloadType::Private => {
            if voteplan.committee_public_keys().is_empty() {
                return Err(Error::InvalidPrivateVotePlanCommitteeKeys);
            }
        }
    }
    Ok(())
}

impl VotePlanRegistration {
    pub fn exec(self) -> Result<(), Error> {
        let configuration = io::open_file_read(&self.input)?;
        let vote_plan_certificate: VotePlanConfiguration =
            serde_yaml::from_reader(configuration).map_err(Error::VotePlanConfig)?;
        validate_voteplan(&vote_plan_certificate.0)?;
        let cert = Certificate::VotePlan(vote_plan_certificate.0);
        write_cert(self.output.as_deref(), cert.into())
    }
}
