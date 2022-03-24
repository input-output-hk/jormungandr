use crate::jcli_lib::{
    certificate::{read_cert, read_input, write_signed_cert, Error},
    utils::key_parser::{self, parse_ed25519_secret_key},
};
use chain_crypto::{Ed25519, PublicKey};
use chain_impl_mockchain::{
    certificate::{
        BftLeaderBindingSignature, Certificate, EvmMapping, PoolOwnersSigned, PoolRegistration,
        PoolSignature, SignedCertificate, StakeDelegation, TallyProof, UpdateProposal, UpdateVote,
        VotePlan, VotePlanProof, VoteTally,
    },
    key::EitherEd25519SecretKey,
    transaction::{
        AccountBindingSignature, Payload, SetAuthData, SingleAccountBindingSignature, Transaction,
        TxBuilderState,
    },
};
use jormungandr_lib::interfaces;
use std::{convert::TryInto, path::PathBuf};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Sign {
    /// path to the file with the signing key
    #[structopt(short = "k", long = "key")]
    pub signing_keys: Vec<PathBuf>,
    /// get the certificate to sign from the given file. If no file
    /// provided, it will be read from the standard input
    #[structopt(short = "c", long = "certificate")]
    pub input: Option<PathBuf>,
    /// write the signed certificate into the given file. If no file
    /// provided it will be written into the standard output
    #[structopt(short = "o", long = "output")]
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
                let txbuilder = Transaction::block0_payload_builder(&s);
                keys_str
                    .len()
                    .eq(&1)
                    .then(|| stake_delegation_account_binding_sign(s, &keys_str[0], txbuilder))
                    .ok_or(Error::ExpectingOnlyOneSigningKey {
                        got: keys_str.len(),
                    })??
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
                keys_str
                    .len()
                    .eq(&1)
                    .then(|| committee_vote_tally_sign(vt, &keys_str[0], txbuilder))
                    .ok_or(Error::ExpectingOnlyOneSigningKey {
                        got: keys_str.len(),
                    })??
            }
            Certificate::OwnerStakeDelegation(_) => {
                return Err(Error::OwnerStakeDelegationDoesntNeedSignature)
            }
            Certificate::VotePlan(vp) => {
                let txbuilder = Transaction::block0_payload_builder(&vp);
                keys_str
                    .len()
                    .eq(&1)
                    .then(|| committee_vote_plan_sign(vp, &keys_str[0], txbuilder))
                    .ok_or(Error::ExpectingOnlyOneSigningKey {
                        got: keys_str.len(),
                    })??
            }
            Certificate::VoteCast(_) => return Err(Error::VoteCastDoesntNeedSignature),
            Certificate::UpdateProposal(up) => {
                let txbuilder = Transaction::block0_payload_builder(&up);
                keys_str
                    .len()
                    .eq(&1)
                    .then(|| update_proposal_sign(up, &keys_str[0], txbuilder))
                    .ok_or(Error::ExpectingOnlyOneSigningKey {
                        got: keys_str.len(),
                    })??
            }
            Certificate::UpdateVote(uv) => {
                let txbuilder = Transaction::block0_payload_builder(&uv);
                keys_str
                    .len()
                    .eq(&1)
                    .then(|| update_vote_sign(uv, &keys_str[0], txbuilder))
                    .ok_or(Error::ExpectingOnlyOneSigningKey {
                        got: keys_str.len(),
                    })??
            }
            Certificate::MintToken(_) => return Err(Error::MintTokenDoesntNeedSignature),
            Certificate::EvmMapping(uv) => {
                let txbuilder = Transaction::block0_payload_builder(&uv);
                keys_str
                    .len()
                    .eq(&1)
                    .then(|| evm_mapping_sign(uv, &keys_str[0], txbuilder))
                    .ok_or(Error::ExpectingOnlyOneSigningKey {
                        got: keys_str.len(),
                    })??
            }
        };
        write_signed_cert(self.output.as_deref(), signedcert.into())
    }
}

pub(crate) fn committee_vote_tally_sign(
    vote_tally: VoteTally,
    key_str: &str,
    builder: TxBuilderState<SetAuthData<VoteTally>>,
) -> Result<SignedCertificate, Error> {
    use chain_impl_mockchain::vote::PayloadType;

    let private_key = parse_ed25519_secret_key(key_str.trim())?;
    let id = private_key.to_public().as_ref().try_into().unwrap();

    let signature = SingleAccountBindingSignature::new(&builder.get_auth_data(), |d| {
        private_key.sign_slice(d.0)
    });

    let proof = match vote_tally.tally_type() {
        PayloadType::Public => TallyProof::Public { id, signature },
        PayloadType::Private => TallyProof::Private { id, signature },
    };
    Ok(SignedCertificate::VoteTally(vote_tally, proof))
}

pub(crate) fn committee_vote_plan_sign(
    vote_plan: VotePlan,
    key_str: &str,
    builder: TxBuilderState<SetAuthData<VotePlan>>,
) -> Result<SignedCertificate, Error> {
    let private_key = parse_ed25519_secret_key(key_str.trim())?;
    let id = private_key.to_public().as_ref().try_into().unwrap();

    let signature = SingleAccountBindingSignature::new(&builder.get_auth_data(), |d| {
        private_key.sign_slice(d.0)
    });

    let proof = VotePlanProof { id, signature };

    Ok(SignedCertificate::VotePlan(vote_plan, proof))
}

pub(crate) fn stake_delegation_account_binding_sign(
    delegation: StakeDelegation,
    key_str: &str,
    builder: TxBuilderState<SetAuthData<StakeDelegation>>,
) -> Result<SignedCertificate, Error> {
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
        private_key.sign_slice(d.0)
    });

    Ok(SignedCertificate::StakeDelegation(delegation, sig))
}

pub(crate) fn pool_owner_sign<F, P: Payload>(
    payload: P,
    mreg: Option<&PoolRegistration>, // if present we verify the secret key against the expectations
    keys_str: &[String],
    builder: TxBuilderState<SetAuthData<P>>,
    to_signed_certificate: F,
) -> Result<SignedCertificate, Error>
where
    F: FnOnce(P, PoolOwnersSigned) -> SignedCertificate,
{
    let keys: Result<Vec<EitherEd25519SecretKey>, key_parser::Error> = keys_str
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
        let sig = SingleAccountBindingSignature::new(&auth_data, |d| key.sign_slice(d.0));
        sigs.push((*i, sig))
    }
    let sig = PoolOwnersSigned { signatures: sigs };
    Ok(to_signed_certificate(payload, sig))
}

pub(crate) fn update_proposal_sign<P: Payload>(
    update_proposal: UpdateProposal,
    key_str: &str,
    builder: TxBuilderState<SetAuthData<P>>,
) -> Result<SignedCertificate, Error> {
    let private_key = parse_ed25519_secret_key(key_str.trim())?;
    let signature =
        BftLeaderBindingSignature::new(&builder.get_auth_data(), |d| private_key.sign_slice(d.0));

    Ok(SignedCertificate::UpdateProposal(
        update_proposal,
        signature,
    ))
}

pub(crate) fn update_vote_sign<P: Payload>(
    update_vote: UpdateVote,
    key_str: &str,
    builder: TxBuilderState<SetAuthData<P>>,
) -> Result<SignedCertificate, Error> {
    let private_key = parse_ed25519_secret_key(key_str.trim())?;

    let signature =
        BftLeaderBindingSignature::new(&builder.get_auth_data(), |d| private_key.sign_slice(d.0));

    Ok(SignedCertificate::UpdateVote(update_vote, signature))
}

pub(crate) fn evm_mapping_sign(
    evm_mapping: EvmMapping,
    key_str: &str,
    builder: TxBuilderState<SetAuthData<EvmMapping>>,
) -> Result<SignedCertificate, Error> {
    let private_key = parse_ed25519_secret_key(key_str.trim())?;

    let signature = SingleAccountBindingSignature::new(&builder.get_auth_data(), |d| {
        private_key.sign_slice(d.0)
    });

    Ok(SignedCertificate::EvmMapping(evm_mapping, signature))
}
