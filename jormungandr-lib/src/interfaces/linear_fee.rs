use chain_impl_mockchain::fee::{LinearFee, PerCertificateFee};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case", remote = "LinearFee")]
pub struct LinearFeeDef {
    constant: u64,
    coefficient: u64,
    certificate: u64,
    #[serde(skip, default = "per_certificate_fees_default")]
    per_certificate_fees: Option<PerCertificateFee>,
}

fn per_certificate_fees_default() -> Option<PerCertificateFee> {
    None
}
