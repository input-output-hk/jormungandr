use chain_impl_mockchain::fee::{LinearFee, PerCertificateFee, PerVoteCertificateFee};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    remote = "PerCertificateFee"
)]
pub struct PerCertificateFeeDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate_pool_registration: Option<NonZeroU64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate_stake_delegation: Option<NonZeroU64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate_owner_stake_delegation: Option<NonZeroU64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    remote = "PerVoteCertificateFee"
)]
pub struct PerVoteCertificateFeeDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate_vote_plan: Option<NonZeroU64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate_vote_cast: Option<NonZeroU64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case", remote = "LinearFee")]
pub struct LinearFeeDef {
    constant: u64,
    coefficient: u64,
    certificate: u64,
    #[serde(
        default,
        with = "PerCertificateFeeDef",
        skip_serializing_if = "per_certificate_fee_is_zero"
    )]
    per_certificate_fees: PerCertificateFee,
    #[serde(
        default,
        with = "PerVoteCertificateFeeDef",
        skip_serializing_if = "per_vote_certificate_fee_is_zero"
    )]
    per_vote_certificate_fees: PerVoteCertificateFee,
}

pub(crate) fn per_certificate_fee_is_zero(fee: &PerCertificateFee) -> bool {
    fee.certificate_stake_delegation.is_none()
        && fee.certificate_owner_stake_delegation.is_none()
        && fee.certificate_pool_registration.is_none()
}

pub(crate) fn per_vote_certificate_fee_is_zero(fee: &PerVoteCertificateFee) -> bool {
    fee.certificate_vote_plan.is_none() && fee.certificate_vote_cast.is_none()
}
