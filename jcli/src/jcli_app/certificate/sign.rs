use chain_crypto::{Ed25519, PublicKey};
use chain_impl_mockchain::certificate::{
    Certificate, PoolOwnersSigned, PoolRegistration, SignedCertificate, StakeDelegation,
};
use chain_impl_mockchain::key::EitherEd25519SecretKey;
use chain_impl_mockchain::transaction::{
    AccountBindingSignature, Payload, SetAuthData, Transaction, TxBuilderState,
};
use jcli_app::certificate::{read_cert, read_input, write_signed_cert, Error};
use jcli_app::utils::key_parser::{self, parse_ed25519_secret_key};
use jormungandr_lib::interfaces;
use std::ops::Deref;
use std::path::PathBuf;
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
        let cert: interfaces::Certificate =
            read_cert(self.input.as_ref().map(|x| x.deref()))?.into();

        if self.signing_keys.len() == 0 {
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
                    SignedCertificate::PoolRegistration(c, a)
                })?
            }
            Certificate::PoolRetirement(s) => {
                let txbuilder = Transaction::block0_payload_builder(&s);
                pool_owner_sign(s, None, &keys_str, txbuilder, |c, a| {
                    SignedCertificate::PoolRetirement(c, a)
                })?
            }
            Certificate::PoolUpdate(s) => {
                let txbuilder = Transaction::block0_payload_builder(&s);
                pool_owner_sign(s, None, &keys_str, txbuilder, |c, a| {
                    SignedCertificate::PoolUpdate(c, a)
                })?
            }
            Certificate::OwnerStakeDelegation(_) => {
                return Err(Error::OwnerStakeDelegationDoesntNeedSignature)
            }
        };
        write_signed_cert(self.output.as_ref().map(|x| x.deref()), signedcert.into())
    }
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
            if &cert_pk != &pk {
                return Err(Error::KeyNotFound { index: 0 });
            }
        }
    }

    let sig = AccountBindingSignature::new(&private_key, &builder.get_auth_data());

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

    let keys: Vec<(u16, &EitherEd25519SecretKey)> = match mreg {
        None => {
            // here we don't know the order of things, so just assume sequential index from 0
            keys.iter()
                .enumerate()
                .map(|(i, k)| (i as u16, k))
                .collect()
        }
        Some(reg) => {
            //let pks = &reg.owners;
            let mut found = Vec::new();
            for (isk, k) in keys.iter().enumerate() {
                let pk = k.to_public();
                // look for the owner's index of k
                match reg.owners.iter().enumerate().find(|(_, p)| *p == &pk) {
                    None => return Err(Error::KeyNotFound { index: isk }),
                    Some((ipk, _)) => found.push((ipk as u16, k)),
                }
            }
            found
        }
    };

    //let txbuilder = Transaction::block0_payload_builder(&payload);
    let auth_data = builder.get_auth_data();

    let mut sigs = Vec::new();
    for (i, key) in keys.iter() {
        let sig = AccountBindingSignature::new(key, &auth_data);
        sigs.push((*i, sig))
    }
    let sig = PoolOwnersSigned { signatures: sigs };
    Ok(to_signed_certificate(payload, sig))
}
