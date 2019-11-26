use chain_impl_mockchain::fee::{LinearFee, PerCertificateFee};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    remote = "PerCertificateFee"
)]
pub struct PerCertificateFeeDef {
    pub certificate_pool_registration: u64,
    pub certificate_stake_delegation: u64,
    pub certificate_owner_stake_delegation: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case", remote = "LinearFee")]
pub struct LinearFeeDef {
    constant: u64,
    coefficient: u64,
    certificate: u64,
    #[serde(
        default,
        with = "opt_per_certificate_fee",
        skip_serializing_if = "Option::is_none"
    )]
    per_certificate_fees: Option<PerCertificateFee>,
}

/// This is a workaround required because currently there is no way to
/// deserialize the `per_certificate_fees` as `Option<PerCertificateFeeDef>`
/// (the required type is `Option<PerCertificateFee>` as specified in the remote
/// `LinearFee` struct). At the same time, there is no way to use `serde_derive`
/// for container types like `Option<T>` where `T` is a remote-derived type.
/// Thus we are required to implement serializers and deserializers for such
/// cases by hand.
///
/// The below is based on
/// https://github.com/serde-rs/serde/issues/1301#issuecomment-394108486
///
/// Fancy implementation of such serialization with `derive` depends on
/// https://github.com/serde-rs/serde/issues/723
mod opt_per_certificate_fee {
    use super::{PerCertificateFee, PerCertificateFeeDef};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(value: &Option<PerCertificateFee>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Helper<'a>(#[serde(with = "PerCertificateFeeDef")] &'a PerCertificateFee);

        value.as_ref().map(Helper).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<PerCertificateFee>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper(#[serde(with = "PerCertificateFeeDef")] PerCertificateFee);

        let helper = Option::deserialize(deserializer)?;
        Ok(helper.map(|Helper(external)| external))
    }
}
