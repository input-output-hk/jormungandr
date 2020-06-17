use crate::jcli_app::{
    certificate::{write_cert, Error},
    utils::io,
};
use chain_impl_mockchain::certificate::{Certificate, VotePlan};
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

impl VotePlanRegistration {
    pub fn exec(self) -> Result<(), Error> {
        let configuration = io::open_file_read(&self.input)?;
        let vote_plan_certificate: VotePlanConfiguration =
            serde_yaml::from_reader(configuration).map_err(Error::VotePlanConfig)?;
        let cert = Certificate::VotePlan(vote_plan_certificate.0);
        write_cert(self.output.as_deref(), cert.into())
    }
}
