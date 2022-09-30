use serde::Deserialize;
use thor::WalletAlias;

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum CommitteeTemplate {
    Generated {
        alias: WalletAlias,
        member_pk: Option<String>,
        communication_pk: Option<String>,
    },
    External {
        id: String,
        member_pk: Option<String>,
        communication_pk: Option<String>,
    },
}
