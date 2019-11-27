use crate::certificate::CertificateSlice;
use crate::transaction as tx;
use crate::value::Value;
use std::num::NonZeroU64;

/// Linear fee using the basic affine formula
/// `COEFFICIENT * bytes(COUNT(tx.inputs) + COUNT(tx.outputs)) + CONSTANT + CERTIFICATE*COUNT(certificates)`.
#[derive(PartialEq, Eq, PartialOrd, Debug, Clone, Copy)]
pub struct LinearFee {
    pub constant: u64,
    pub coefficient: u64,
    pub certificate: u64,
    pub per_certificate_fees: PerCertificateFee,
}

#[derive(PartialEq, Eq, PartialOrd, Debug, Clone, Copy, Default)]
pub struct PerCertificateFee {
    pub certificate_pool_registration: Option<NonZeroU64>,
    pub certificate_stake_delegation: Option<NonZeroU64>,
    pub certificate_owner_stake_delegation: Option<NonZeroU64>,
}

impl LinearFee {
    pub fn new(constant: u64, coefficient: u64, certificate: u64) -> Self {
        LinearFee {
            constant,
            coefficient,
            certificate,
            per_certificate_fees: PerCertificateFee::default(),
        }
    }

    pub fn per_certificate_fees(&mut self, per_certificate_fees: PerCertificateFee) {
        self.per_certificate_fees = per_certificate_fees;
    }
}

impl PerCertificateFee {
    pub fn new(
        certificate_pool_registration: Option<NonZeroU64>,
        certificate_stake_delegation: Option<NonZeroU64>,
        certificate_owner_stake_delegation: Option<NonZeroU64>,
    ) -> Self {
        Self {
            certificate_pool_registration,
            certificate_stake_delegation,
            certificate_owner_stake_delegation,
        }
    }

    fn fees_for_certificate<'a>(&self, cert: CertificateSlice<'a>) -> Option<Value> {
        match cert {
            CertificateSlice::PoolRegistration(_) => {
                self.certificate_pool_registration.map(|v| Value(v.get()))
            }
            CertificateSlice::StakeDelegation(_) => {
                self.certificate_stake_delegation.map(|v| Value(v.get()))
            }
            CertificateSlice::OwnerStakeDelegation(_) => self
                .certificate_owner_stake_delegation
                .map(|v| Value(v.get())),
            _ => None,
        }
    }
}

pub trait FeeAlgorithm {
    fn baseline(&self) -> Value;
    fn fees_for_inputs_outputs(&self, inputs: u8, outputs: u8) -> Value;
    fn fees_for_certificate<'a>(&self, cert: CertificateSlice<'a>) -> Value;

    fn calculate<'a>(&self, cert: Option<CertificateSlice<'a>>, inputs: u8, outputs: u8) -> Value {
        self.baseline()
            .saturating_add(self.fees_for_inputs_outputs(inputs, outputs))
            .saturating_add(cert.map_or(Value::zero(), |c| self.fees_for_certificate(c)))
    }

    fn calculate_tx<P: tx::Payload>(&self, tx: &tx::Transaction<P>) -> Value {
        self.calculate(
            tx.as_slice().payload().to_certificate_slice(),
            tx.nb_inputs(),
            tx.nb_outputs(),
        )
    }
}

impl FeeAlgorithm for LinearFee {
    fn baseline(&self) -> Value {
        Value(self.constant)
    }

    fn fees_for_inputs_outputs(&self, inputs: u8, outputs: u8) -> Value {
        Value(
            self.coefficient
                .saturating_mul((inputs as u64) + (outputs as u64)),
        )
    }

    fn fees_for_certificate<'a>(&self, cert_slice: CertificateSlice<'a>) -> Value {
        self.per_certificate_fees
            .fees_for_certificate(cert_slice)
            .unwrap_or(Value(self.certificate))
    }
}

#[cfg(any(test, feature = "property-test-api"))]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for PerCertificateFee {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            PerCertificateFee::new(
                NonZeroU64::new(u64::arbitrary(g)),
                NonZeroU64::new(u64::arbitrary(g)),
                NonZeroU64::new(u64::arbitrary(g)),
            )
        }
    }

    impl Arbitrary for LinearFee {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Self {
                constant: Arbitrary::arbitrary(g),
                coefficient: Arbitrary::arbitrary(g),
                certificate: Arbitrary::arbitrary(g),
                per_certificate_fees: PerCertificateFee::new(None, None, None),
            }
        }
    }
}
