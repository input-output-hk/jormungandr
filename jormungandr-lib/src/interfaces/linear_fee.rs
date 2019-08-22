use chain_impl_mockchain::fee::LinearFee;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case", remote = "LinearFee")]
pub struct LinearFeeDef {
    constant: u64,
    coefficient: u64,
    certificate: u64,
}
