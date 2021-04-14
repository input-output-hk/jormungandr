use crate::{
    certificate::{read_cert, read_input, write_signed_cert, Error},
    utils::key_parser::{self, parse_ed25519_secret_key},
};
use chain_crypto::{Ed25519, PublicKey};
use chain_impl_mockchain::certificate::{EncryptedVoteTally, EncryptedVoteTallyProof};
use chain_impl_mockchain::{
    certificate::{
        Certificate, PoolOwnersSigned, PoolRegistration, PoolSignature, SignedCertificate,
        StakeDelegation, TallyProof, VotePlan, VotePlanProof, VoteTally,
    },
    key::EitherEd25519SecretKey,
    transaction::{
        AccountBindingSignature, Payload, SetAuthData, SingleAccountBindingSignature, Transaction,
        TxBuilderState,
    },
};
use jormungandr_lib::interfaces;
use std::{convert::TryInto, path::PathBuf};
#[cfg(feature = "structopt")]
use structopt::StructOpt;

#[cfg_attr(
    feature = "structopt",
    derive(StructOpt),
    structopt(rename_all = "kebab-case")
)]
pub struct Sign {
    /// path to the file with the signing key
    #[cfg_attr(feature = "structopt", structopt(short = "k", long = "key"))]
    pub signing_keys: Vec<PathBuf>,
    /// get the certificate to sign from the given file. If no file
    /// provided, it will be read from the standard input
    #[cfg_attr(feature = "structopt", structopt(short = "c", long = "certificate"))]
    pub input: Option<PathBuf>,
    /// write the signed certificate into the given file. If no file
    /// provided it will be written into the standard output
    #[cfg_attr(feature = "structopt", structopt(short = "o", long = "output"))]
    pub output: Option<PathBuf>,
}

impl Sign {
    pub fn exec(self) -> Result<(), Error> {
        let cert: interfaces::Certificate = read_cert(self.input.as_deref())?;

        if self.signing_keys.is_empty() {
            return Err(Error::NoSigningKeys);
        }

        let keys_str: Result<Vec<String>, Error> = self
            .signing_keys
            .iter()
            .map(|sk| read_input(Some(sk.as_ref())))
            .collect();
        let keys_str = keys_str?;

        let signedcert = match cert.into() {
            Certificate::StakeDelegation(s) => {
                let builder = Transaction::block0_payload_builder(&s);
                stake_delegation_account_binding_sign(s, &keys_str, builder)?
            }
            Certificate::PoolRegistration(s) => {
                let sclone = s.clone();
                let txbuilder = Transaction::block0_payload_builder(&s);
                pool_owner_sign(s, Some(&sclone), &keys_str, txbuilder, |c, a| {
                    SignedCertificate::PoolRegistration(c, PoolSignature::Owners(a))
                })?
            }
            Certificate::PoolRetirement(s) => {
                let txbuilder = Transaction::block0_payload_builder(&s);
                pool_owner_sign(s, None, &keys_str, txbuilder, |c, a| {
                    SignedCertificate::PoolRetirement(c, PoolSignature::Owners(a))
                })?
            }
            Certificate::PoolUpdate(s) => {
                let txbuilder = Transaction::block0_payload_builder(&s);
                pool_owner_sign(s, None, &keys_str, txbuilder, |c, a| {
                    SignedCertificate::PoolUpdate(c, PoolSignature::Owners(a))
                })?
            }
            Certificate::VoteTally(vt) => {
                let txbuilder = Transaction::block0_payload_builder(&vt);
                committee_vote_tally_sign(vt, &keys_str, txbuilder)?
            }
            Certificate::EncryptedVoteTally(vt) => {
                let txbuilder = Transaction::block0_payload_builder(&vt);
                committee_encrypted_vote_tally_sign(vt, &keys_str, txbuilder)?
            }
            Certificate::OwnerStakeDelegation(_) => {
                return Err(Error::OwnerStakeDelegationDoesntNeedSignature)
            }
            Certificate::VotePlan(vp) => {
                let txbuilder = Transaction::block0_payload_builder(&vp);
                committee_vote_plan_sign(vp, &keys_str, txbuilder)?
            }
            Certificate::VoteCast(_) => return Err(Error::VoteCastDoesntNeedSignature),
        };
        write_signed_cert(self.output.as_deref(), signedcert.into())
    }
}

pub(crate) fn committee_vote_tally_sign(
    vote_tally: VoteTally,
    keys_str: &[String],
    builder: TxBuilderState<SetAuthData<VoteTally>>,
) -> Result<SignedCertificate, Error> {
    use chain_impl_mockchain::vote::PayloadType;

    if keys_str.len() > 1 {
        return Err(Error::ExpectingOnlyOneSigningKey {
            got: keys_str.len(),
        });
    }
    let private_key = parse_ed25519_secret_key(keys_str[0].trim())?;
    let id = private_key.to_public().as_ref().try_into().unwrap();

    let signature = SingleAccountBindingSignature::new(&builder.get_auth_data(), |d| {
        private_key.sign_slice(&d.0)
    });

    let proof = match vote_tally.tally_type() {
        PayloadType::Public => TallyProof::Public { id, signature },
        PayloadType::Private => TallyProof::Private { id, signature },
    };
    Ok(SignedCertificate::VoteTally(vote_tally, proof))
}

pub(crate) fn committee_encrypted_vote_tally_sign(
    vote_tally: EncryptedVoteTally,
    keys_str: &[String],
    builder: TxBuilderState<SetAuthData<EncryptedVoteTally>>,
) -> Result<SignedCertificate, Error> {
    if keys_str.len() > 1 {
        return Err(Error::ExpectingOnlyOneSigningKey {
            got: keys_str.len(),
        });
    }
    let private_key = parse_ed25519_secret_key(keys_str[0].trim())?;
    let id = private_key.to_public().as_ref().try_into().unwrap();

    let signature = SingleAccountBindingSignature::new(&builder.get_auth_data(), |d| {
        private_key.sign_slice(&d.0)
    });

    let proof = EncryptedVoteTallyProof { id, signature };
    Ok(SignedCertificate::EncryptedVoteTally(vote_tally, proof))
}

pub(crate) fn committee_vote_plan_sign(
    vote_plan: VotePlan,
    keys_str: &[String],
    builder: TxBuilderState<SetAuthData<VotePlan>>,
) -> Result<SignedCertificate, Error> {
    if keys_str.len() > 1 {
        return Err(Error::ExpectingOnlyOneSigningKey {
            got: keys_str.len(),
        });
    }

    let private_key = parse_ed25519_secret_key(keys_str[0].trim())?;
    let id = private_key.to_public().as_ref().try_into().unwrap();

    let signature = SingleAccountBindingSignature::new(&builder.get_auth_data(), |d| {
        private_key.sign_slice(&d.0)
    });

    let proof = VotePlanProof { id, signature };

    Ok(SignedCertificate::VotePlan(vote_plan, proof))
}

pub(crate) fn stake_delegation_account_binding_sign(
    delegation: StakeDelegation,
    keys_str: &[String],
    builder: TxBuilderState<SetAuthData<StakeDelegation>>,
) -> Result<SignedCertificate, Error> {
    if keys_str.len() > 1 {
        return Err(Error::ExpectingOnlyOneSigningKey {
            got: keys_str.len(),
        });
    }
    let key_str = keys_str[0].clone();
    let private_key = parse_ed25519_secret_key(key_str.trim())?;

    // check that it match the stake delegation account
    match delegation.account_id.to_single_account() {
        None => {}
        Some(acid) => {
            let pk = private_key.to_public();
            let cert_pk: PublicKey<Ed25519> = acid.into();
            if cert_pk != pk {
                return Err(Error::KeyNotFound { index: 0 });
            }
        }
    }

    let sig = AccountBindingSignature::new_single(&builder.get_auth_data(), |d| {
        private_key.sign_slice(&d.0)
    });

    Ok(SignedCertificate::StakeDelegation(delegation, sig))
}

pub(crate) fn pool_owner_sign<F, P: Payload>(
    payload: P,
    mreg: Option<&PoolRegistration>, // if present we verify the secret key against the expectations
    keys: &[String],
    builder: TxBuilderState<SetAuthData<P>>,
    to_signed_certificate: F,
) -> Result<SignedCertificate, Error>
where
    F: FnOnce(P, PoolOwnersSigned) -> SignedCertificate,
{
    let keys: Result<Vec<EitherEd25519SecretKey>, key_parser::Error> = keys
        .iter()
        .map(|sk| parse_ed25519_secret_key(sk.clone().trim()))
        .collect();
    let keys = keys?;

    let keys: Vec<(u8, &EitherEd25519SecretKey)> = match mreg {
        None => {
            // here we don't know the order of things, so just assume sequential index from 0
            keys.iter().enumerate().map(|(i, k)| (i as u8, k)).collect()
        }
        Some(reg) => {
            //let pks = &reg.owners;
            let mut found = Vec::new();
            for (isk, k) in keys.iter().enumerate() {
                let pk = k.to_public();
                // look for the owner's index of k
                match reg.owners.iter().enumerate().find(|(_, p)| *p == &pk) {
                    None => return Err(Error::KeyNotFound { index: isk }),
                    Some((ipk, _)) => found.push((ipk as u8, k)),
                }
            }
            found
        }
    };

    //let txbuilder = Transaction::block0_payload_builder(&payload);
    let auth_data = builder.get_auth_data();

    let mut sigs = Vec::new();
    for (i, key) in keys.iter() {
        let sig = SingleAccountBindingSignature::new(&auth_data, |d| key.sign_slice(&d.0));
        sigs.push((*i, sig))
    }
    let sig = PoolOwnersSigned { signatures: sigs };
    Ok(to_signed_certificate(payload, sig))
}
