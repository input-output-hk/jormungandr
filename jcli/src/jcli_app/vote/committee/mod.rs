mod communication_key;
mod member_key;

use super::Error;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Committee {
    /// generate a private key
    CommunicationKey(communication_key::CommunicationKey),
    /// get the public key out of a given private key
    MemberKey(member_key::MemberKey),
}

impl Committee {
    pub fn exec(self) -> Result<(), super::Error> {
        match self {
            Committee::CommunicationKey(args) => args.exec(),
            Committee::MemberKey(args) => args.exec(),
        }
    }
}
