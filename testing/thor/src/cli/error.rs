use super::config::Alias;
use crate::{FragmentSenderError, FragmentVerifierError};
use chain_crypto::SecretKeyError;
use chain_impl_mockchain::fragment::FragmentId;
use jormungandr_automation::jormungandr::RestError;
use jormungandr_lib::crypto::account::SigningKeyParseError;
use thiserror::Error;
#[derive(Debug, Error)]
#[allow(clippy::large_enum_variant)]
pub enum Error {
    #[error(transparent)]
    FragmentVerifier(#[from] FragmentVerifierError),
    #[error(transparent)]
    FragmentSender(#[from] FragmentSenderError),
    #[error(transparent)]
    Bech32(#[from] bech32::Error),
    #[error(transparent)]
    SecretKey(#[from] SecretKeyError),
    #[error("cannot connect to backend under address: {0}, due to: {1:?}")]
    Connection(String, RestError),
    #[error(transparent)]
    SigningKeyParse(#[from] SigningKeyParseError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serde(#[from] serde_yaml::Error),
    #[error(transparent)]
    Config(#[from] crate::cli::config::Error),
    #[error("cannot serialize secret key")]
    CannotrSerializeSecretKey,
    #[error("cannot create spending counter")]
    SpendingCounter,

    #[error("cannot read secret key")]
    CannotReadSecretKey,
    #[error("unknown alias: '{0}'")]
    UknownAlias(Alias),
    #[error("no default alias specified")]
    NoDefaultAliasDefined,
    #[error("cannot read/write secret key")]
    Cocoon,
    #[error("Bincode error")]
    Bincode,
    #[error(transparent)]
    Key(#[from] jcli_lib::key::Error),

    #[error("cannot find proposal: voteplan({vote_plan_name}) index({proposal_index})")]
    CannotFindProposal {
        vote_plan_name: String,
        proposal_index: u32,
    },
    #[error("transactions with ids [{fragments:?}] were pending for too long")]
    TransactionsWerePendingForTooLong { fragments: Vec<FragmentId> },
    #[error(transparent)]
    Rest(#[from] RestError),
}

impl From<cocoon::Error> for Error {
    fn from(_err: cocoon::Error) -> Self {
        Error::Cocoon
    }
}
