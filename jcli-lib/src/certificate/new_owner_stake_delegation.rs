use crate::certificate::{weighted_pool_ids::WeightedPoolIds, write_cert, Error};
use chain_impl_mockchain::certificate::{Certificate, OwnerStakeDelegation as Delegation};
use jormungandr_lib::interfaces::Certificate as CertificateType;
use std::{convert::TryInto, path::PathBuf};
#[cfg(feature = "structopt")]
use structopt::StructOpt;

#[cfg_attr(feature = "structopt", derive(StructOpt))]
pub struct OwnerStakeDelegation {
    #[cfg_attr(feature = "structopt", structopt(flatten))]
    pool_ids: WeightedPoolIds,

    /// write the output to the given file or print it to the standard output if not defined
    #[cfg_attr(feature = "structopt", structopt(short = "o", long = "output"))]
    output: Option<PathBuf>,
}

impl OwnerStakeDelegation {
    pub fn exec(self) -> Result<(), Error> {
        let cert = Certificate::OwnerStakeDelegation(Delegation {
            delegation: (&self.pool_ids).try_into()?,
        });
        write_cert(self.output.as_deref(), CertificateType(cert))
    }
}
