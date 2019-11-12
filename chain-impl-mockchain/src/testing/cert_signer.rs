
use crate::{
    certificate::{Certificate,SignedCertificate,PoolOwnersSigned,PoolSignature},
    transaction::{SetAuthData,TxBuilderState,AccountBindingSignature,Payload,Transaction,SingleAccountBindingSignature},
    key::EitherEd25519SecretKey
};

custom_error! {
    #[derive(Clone, PartialEq, Eq)]
    pub CertificateSignerError
        MoreThanOneKeyUsed = "more than 1 key is provided",
        NoKeys = "no keys provided",
        KeyNotFound { index: usize } = "secret key number {index} matching the expected public key has not been found",
        NoCertificate = "No certificate to  sign",
        NoNeedToSign { name: String } = "{name} type does not need signing"
}

pub struct CertificateSigner{
    keys: Vec<EitherEd25519SecretKey>,
    certificate: Option<Certificate>,
}

impl CertificateSigner {
    pub fn new() -> Self {
        CertificateSigner{
            keys: Vec::new(),
            certificate: None
        }
    }

    pub fn with_certificate(&mut self, certificate: &Certificate) -> &mut Self {
        self.certificate = Some(certificate.clone());
        self
    }

    pub fn with_key(&mut self, key: EitherEd25519SecretKey) -> &mut Self {
        self.keys.push(key.clone());
        self
    }
    
    pub fn with_keys(&mut self, keys: Vec<EitherEd25519SecretKey>) -> &mut Self {
        self.keys.extend(keys.iter().cloned());
        self
    }

    pub fn sign(&self) -> Result<SignedCertificate,CertificateSignerError> {
        let certificate = self.certificate.clone().ok_or(CertificateSignerError::NoCertificate{})?;

        if self.keys.is_empty() {
            return Err(CertificateSignerError::NoKeys{});
        }

        self.sign_certificate(certificate,&self.keys)
    }

    fn sign_certificate(&self, certificate: Certificate, keys: &[EitherEd25519SecretKey]) -> Result<SignedCertificate,CertificateSignerError> {
        match certificate {
                Certificate::StakeDelegation(s) => {
                    if keys.len() > 1 {
                        return Err(CertificateSignerError::MoreThanOneKeyUsed{});
                    }
                    let builder = Transaction::block0_payload_builder(&s);
                    let signature = AccountBindingSignature::new_single(&keys[0], &builder.get_auth_data());
                    Ok(SignedCertificate::StakeDelegation(s.clone(), signature))
                },
                Certificate::PoolRegistration(s) => {
                    let builder = Transaction::block0_payload_builder(&s);
                    let signature = self.pool_owner_sign(&keys, builder);
                    Ok(SignedCertificate::PoolRegistration(s.clone(),signature))
                },
                Certificate::PoolRetirement(s) => {
                    let builder = Transaction::block0_payload_builder(&s);
                    let signature = self.pool_owner_sign(&keys, builder);
                    Ok(SignedCertificate::PoolRetirement(s.clone(),signature))
                },
                Certificate::PoolUpdate(s) => {
                    let builder = Transaction::block0_payload_builder(&s);
                    let signature = self.pool_owner_sign(&keys, builder);
                    Ok(SignedCertificate::PoolUpdate(s.clone(),signature))
                },
                Certificate::OwnerStakeDelegation(_) => {
                    Err(CertificateSignerError::NoNeedToSign{name: "OwnerStakeDelegation".to_string()})
                }
            }
        }

    fn pool_owner_sign<P: Payload>(
        &self,
        keys: &[EitherEd25519SecretKey],
        builder: TxBuilderState<SetAuthData<P>>
    ) -> PoolSignature {
        let auth_data = builder.get_auth_data();
        let mut sigs = Vec::new();
        for (i, key) in keys.iter().enumerate() {
            let sig = SingleAccountBindingSignature::new(key, &auth_data);
            sigs.push((i as u8, sig))
        }
        let pool_owner = PoolOwnersSigned { signatures: sigs };
        PoolSignature::Owners(pool_owner)
    }
}