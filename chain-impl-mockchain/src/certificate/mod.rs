mod delegation;
mod pool;

#[cfg(test)]
mod test;

use crate::transaction::{Payload, PayloadData, PayloadSlice};

pub use delegation::{OwnerStakeDelegation, StakeDelegation};
pub use pool::{
    IndexSignatures, PoolId, PoolOwnersSigned, PoolPermissions, PoolRegistration, PoolRetirement,
    PoolUpdate,
};

pub enum CertificateSlice<'a> {
    StakeDelegation(PayloadSlice<'a, StakeDelegation>),
    OwnerStakeDelegation(PayloadSlice<'a, OwnerStakeDelegation>),
    PoolRegistration(PayloadSlice<'a, PoolRegistration>),
    PoolRetirement(PayloadSlice<'a, PoolRetirement>),
    PoolUpdate(PayloadSlice<'a, PoolUpdate>),
}

impl<'a> From<PayloadSlice<'a, StakeDelegation>> for CertificateSlice<'a> {
    fn from(payload: PayloadSlice<'a, StakeDelegation>) -> CertificateSlice<'a> {
        CertificateSlice::StakeDelegation(payload)
    }
}

impl<'a> From<PayloadSlice<'a, OwnerStakeDelegation>> for CertificateSlice<'a> {
    fn from(payload: PayloadSlice<'a, OwnerStakeDelegation>) -> CertificateSlice<'a> {
        CertificateSlice::OwnerStakeDelegation(payload)
    }
}

impl<'a> From<PayloadSlice<'a, PoolRegistration>> for CertificateSlice<'a> {
    fn from(payload: PayloadSlice<'a, PoolRegistration>) -> CertificateSlice<'a> {
        CertificateSlice::PoolRegistration(payload)
    }
}
impl<'a> From<PayloadSlice<'a, PoolRetirement>> for CertificateSlice<'a> {
    fn from(payload: PayloadSlice<'a, PoolRetirement>) -> CertificateSlice<'a> {
        CertificateSlice::PoolRetirement(payload)
    }
}

impl<'a> From<PayloadSlice<'a, PoolUpdate>> for CertificateSlice<'a> {
    fn from(payload: PayloadSlice<'a, PoolUpdate>) -> CertificateSlice<'a> {
        CertificateSlice::PoolUpdate(payload)
    }
}

impl<'a> CertificateSlice<'a> {
    pub fn into_owned(self) -> Certificate {
        match self {
            CertificateSlice::PoolRegistration(c) => {
                Certificate::PoolRegistration(c.into_payload())
            }
            CertificateSlice::PoolUpdate(c) => Certificate::PoolUpdate(c.into_payload()),
            CertificateSlice::PoolRetirement(c) => Certificate::PoolRetirement(c.into_payload()),
            CertificateSlice::StakeDelegation(c) => Certificate::StakeDelegation(c.into_payload()),
            CertificateSlice::OwnerStakeDelegation(c) => {
                Certificate::OwnerStakeDelegation(c.into_payload())
            }
        }
    }
}

#[derive(Clone)]
pub enum CertificatePayload {
    StakeDelegation(PayloadData<StakeDelegation>),
    OwnerStakeDelegation(PayloadData<OwnerStakeDelegation>),
    PoolRegistration(PayloadData<PoolRegistration>),
    PoolRetirement(PayloadData<PoolRetirement>),
    PoolUpdate(PayloadData<PoolUpdate>),
}

impl CertificatePayload {
    pub fn as_slice(&self) -> CertificateSlice {
        match self {
            CertificatePayload::StakeDelegation(payload) => payload.borrow().into(),
            CertificatePayload::OwnerStakeDelegation(payload) => payload.borrow().into(),
            CertificatePayload::PoolRegistration(payload) => payload.borrow().into(),
            CertificatePayload::PoolRetirement(payload) => payload.borrow().into(),
            CertificatePayload::PoolUpdate(payload) => payload.borrow().into(),
        }
    }
}

impl<'a> From<&'a Certificate> for CertificatePayload {
    fn from(certificate: &'a Certificate) -> Self {
        match certificate {
            Certificate::StakeDelegation(payload) => {
                CertificatePayload::StakeDelegation(payload.payload_data())
            }
            Certificate::OwnerStakeDelegation(payload) => {
                CertificatePayload::OwnerStakeDelegation(payload.payload_data())
            }
            Certificate::PoolRegistration(payload) => {
                CertificatePayload::PoolRegistration(payload.payload_data())
            }
            Certificate::PoolRetirement(payload) => {
                CertificatePayload::PoolRetirement(payload.payload_data())
            }
            Certificate::PoolUpdate(payload) => {
                CertificatePayload::PoolUpdate(payload.payload_data())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Certificate {
    StakeDelegation(StakeDelegation),
    OwnerStakeDelegation(OwnerStakeDelegation),
    PoolRegistration(PoolRegistration),
    PoolRetirement(PoolRetirement),
    PoolUpdate(PoolUpdate),
}

impl From<StakeDelegation> for Certificate {
    fn from(cert: StakeDelegation) -> Certificate {
        Certificate::StakeDelegation(cert)
    }
}

impl From<OwnerStakeDelegation> for Certificate {
    fn from(cert: OwnerStakeDelegation) -> Certificate {
        Certificate::OwnerStakeDelegation(cert)
    }
}

impl From<PoolRegistration> for Certificate {
    fn from(cert: PoolRegistration) -> Certificate {
        Certificate::PoolRegistration(cert)
    }
}

impl From<PoolRetirement> for Certificate {
    fn from(cert: PoolRetirement) -> Certificate {
        Certificate::PoolRetirement(cert)
    }
}

impl From<PoolUpdate> for Certificate {
    fn from(cert: PoolUpdate) -> Certificate {
        Certificate::PoolUpdate(cert)
    }
}

impl Certificate {
    pub fn need_auth(&self) -> bool {
        match self {
            Certificate::PoolRegistration(_) => <PoolRegistration as Payload>::HAS_AUTH,
            Certificate::PoolUpdate(_) => <PoolUpdate as Payload>::HAS_AUTH,
            Certificate::PoolRetirement(_) => <PoolRetirement as Payload>::HAS_AUTH,
            Certificate::StakeDelegation(_) => <StakeDelegation as Payload>::HAS_AUTH,
            Certificate::OwnerStakeDelegation(_) => <OwnerStakeDelegation as Payload>::HAS_AUTH,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SignedCertificate {
    StakeDelegation(StakeDelegation, <StakeDelegation as Payload>::Auth),
    OwnerStakeDelegation(
        OwnerStakeDelegation,
        <OwnerStakeDelegation as Payload>::Auth,
    ),
    PoolRegistration(PoolRegistration, <PoolRegistration as Payload>::Auth),
    PoolRetirement(PoolRetirement, <PoolRetirement as Payload>::Auth),
    PoolUpdate(PoolUpdate, <PoolUpdate as Payload>::Auth),
}
