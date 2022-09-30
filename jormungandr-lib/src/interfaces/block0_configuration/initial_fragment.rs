use crate::interfaces::{
    mint_token::{MintingPolicy, TokenIdentifier},
    Address, OldAddress, SignedCertificate, Value,
};
use chain_addr::{Discrimination, Kind};
use chain_impl_mockchain::{
    block::BlockDate,
    certificate,
    fragment::Fragment,
    legacy::UtxoDeclaration,
    tokens::identifier,
    transaction::{NoExtra, Output, Payload, Transaction, TransactionSlice, TxBuilder},
};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use thiserror::Error;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Initial {
    Fund(Vec<InitialUTxO>),
    Cert(SignedCertificate),
    LegacyFund(Vec<LegacyUTxO>),
    Token(InitialToken),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialUTxO {
    pub address: Address,
    pub value: Value,
}

impl InitialUTxO {
    pub fn to_output(&self) -> Output<chain_addr::Address> {
        Output {
            address: self.address.clone().into(),
            value: self.value.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyUTxO {
    pub address: OldAddress,
    pub value: Value,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("initial UTXO has input")]
    InitUtxoHasInput,
    #[error("non-first message of block 0 has unexpected type")]
    Block0MessageUnexpected,
    #[error("invalid address type for initializing tokens, should be 'Account' or 'Single' ")]
    TokenInvalidAddressType,
    #[error("invalid token identifier, mintinting policy hash mismatch")]
    TokenIdentifierMintingPolicyHashMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Destination {
    pub address: Address,
    pub value: Value,
}

impl From<InitialUTxO> for Destination {
    fn from(utxo: InitialUTxO) -> Self {
        Self {
            address: utxo.address,
            value: utxo.value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InitialToken {
    pub token_id: TokenIdentifier,
    // TODO add a serde implementation for the MintingPolicy when it will be well specified
    #[serde(skip)]
    pub policy: MintingPolicy,
    pub to: Vec<Destination>,
}

pub fn try_initial_fragment_from_message(
    discrimination: Discrimination,
    message: &Fragment,
) -> Result<Initial, Error> {
    match message {
        Fragment::Transaction(tx) => Ok(try_extend_inits_with_tx(&tx.as_slice())?),
        Fragment::OldUtxoDeclaration(utxo) => Ok(extend_inits_with_legacy_utxo(utxo)),
        Fragment::PoolRegistration(tx) => {
            let tx = tx.as_slice();
            let cert = tx.payload().into_payload();
            let auth = tx.payload_auth().into_payload_auth();
            let cert = certificate::SignedCertificate::PoolRegistration(cert, auth);
            Ok(Initial::Cert(cert.into()))
        }
        Fragment::StakeDelegation(tx) => {
            let tx = tx.as_slice();
            let cert = tx.payload().into_payload();
            let auth = tx.payload_auth().into_payload_auth();
            let cert = certificate::SignedCertificate::StakeDelegation(cert, auth);
            Ok(Initial::Cert(cert.into()))
        }
        Fragment::VotePlan(tx) => {
            let tx = tx.as_slice();
            let cert = tx.payload().into_payload();
            // the pattern match here is to make sure we are actually expecting the `()`
            // and that if it changes the compiler will detect it and tell us about the
            // change so we are reminded of a breaking change
            let auth = tx.payload_auth().into_payload_auth();
            let cert = certificate::SignedCertificate::VotePlan(cert, auth);
            Ok(Initial::Cert(cert.into()))
        }
        Fragment::MintToken(tx) => {
            let tx = tx.as_slice();
            let mint_token = tx.payload().into_payload();
            let token_id = identifier::TokenIdentifier {
                token_name: mint_token.name,
                policy_hash: mint_token.policy.hash(),
            };
            Ok(Initial::Token(InitialToken {
                token_id: token_id.into(),
                policy: mint_token.policy.into(),
                to: vec![Destination {
                    address: chain_addr::Address(
                        discrimination,
                        chain_addr::Kind::Account(mint_token.to.into()),
                    )
                    .into(),
                    value: mint_token.value.into(),
                }],
            }))
        }
        _ => Err(Error::Block0MessageUnexpected),
    }
}

fn try_extend_inits_with_tx(tx: &TransactionSlice<NoExtra>) -> Result<Initial, Error> {
    if tx.nb_inputs() != 0 {
        return Err(Error::InitUtxoHasInput);
    }
    let inits_iter = tx.outputs().iter().map(|output| InitialUTxO {
        address: output.address.clone().into(),
        value: output.value.into(),
    });
    Ok(Initial::Fund(inits_iter.collect()))
}

fn extend_inits_with_legacy_utxo(utxo_decl: &UtxoDeclaration) -> Initial {
    if utxo_decl.addrs.is_empty() {
        panic!("old utxo declaration has no element")
    }
    if utxo_decl.addrs.len() >= 255 {
        panic!(
            "old utxo declaration has too many element {}",
            utxo_decl.addrs.len()
        )
    }

    let inits_iter = utxo_decl.addrs.iter().map(|(address, value)| LegacyUTxO {
        address: address.clone().into(),
        value: (*value).into(),
    });
    let inits: Vec<_> = inits_iter.collect();
    Initial::LegacyFund(inits)
}

impl<'a> TryFrom<&'a Initial> for Vec<Fragment> {
    type Error = Error;
    fn try_from(initial: &'a Initial) -> Result<Self, Self::Error> {
        match initial {
            Initial::Fund(utxo) => Ok(pack_utxo_in_message(utxo)),
            Initial::Cert(cert) => Ok(pack_certificate_in_empty_tx_fragment(cert)),
            Initial::LegacyFund(utxo) => Ok(pack_legacy_utxo_in_message(utxo)),
            Initial::Token(token) => pack_tokens_in_mint_token_fragments(token),
        }
    }
}

fn pack_utxo_in_message(v: &[InitialUTxO]) -> Vec<Fragment> {
    let outputs: Vec<_> = v.iter().map(|utxo| utxo.to_output()).collect();

    if outputs.is_empty() {
        panic!("cannot create a singular transaction fragment with 0 output")
    }
    if outputs.len() >= 255 {
        panic!("cannot create a singular transaction fragment with more than 254 outputs ({} requested). spread outputs to another fragment", outputs.len())
    }

    let valid_until = BlockDate::first();

    let tx = TxBuilder::new()
        .set_nopayload()
        .set_expiry_date(valid_until)
        .set_ios(&[], &outputs[..])
        .set_witnesses(&[])
        .set_payload_auth(&());
    vec![Fragment::Transaction(tx)]
}

fn pack_legacy_utxo_in_message(v: &[LegacyUTxO]) -> Vec<Fragment> {
    if v.is_empty() {
        panic!("cannot create a singular legacy declaration fragment with 0 declaration")
    }
    if v.len() >= 255 {
        panic!("cannot create a singular legacy declaration fragment with more than 254 declarations ({} requested). spread declarations to another fragment", v.len())
    }
    let addrs = v
        .iter()
        .map(|utxo| (utxo.address.clone().into(), utxo.value.into()))
        .collect();
    vec![Fragment::OldUtxoDeclaration(UtxoDeclaration { addrs })]
}

fn empty_auth_tx<P: Payload>(payload: &P, payload_auth: &P::Auth) -> Transaction<P> {
    Transaction::block0_payload(payload, payload_auth)
}

fn pack_certificate_in_empty_tx_fragment(cert: &SignedCertificate) -> Vec<Fragment> {
    vec![match &cert.0 {
        certificate::SignedCertificate::StakeDelegation(c, a) => {
            Fragment::StakeDelegation(empty_auth_tx(c, a))
        }
        certificate::SignedCertificate::OwnerStakeDelegation(c, a) => {
            Fragment::OwnerStakeDelegation(empty_auth_tx(c, a))
        }
        certificate::SignedCertificate::PoolRegistration(c, a) => {
            Fragment::PoolRegistration(empty_auth_tx(c, a))
        }
        certificate::SignedCertificate::PoolRetirement(c, a) => {
            Fragment::PoolRetirement(empty_auth_tx(c, a))
        }
        certificate::SignedCertificate::PoolUpdate(c, a) => {
            Fragment::PoolUpdate(empty_auth_tx(c, a))
        }
        certificate::SignedCertificate::VotePlan(c, a) => Fragment::VotePlan(empty_auth_tx(c, a)),
        certificate::SignedCertificate::VoteTally(c, a) => Fragment::VoteTally(empty_auth_tx(c, a)),
        certificate::SignedCertificate::UpdateProposal(c, a) => {
            Fragment::UpdateProposal(empty_auth_tx(c, a))
        }
        certificate::SignedCertificate::UpdateVote(c, a) => {
            Fragment::UpdateVote(empty_auth_tx(c, a))
        }
        certificate::SignedCertificate::EvmMapping(c, a) => {
            Fragment::EvmMapping(empty_auth_tx(c, a))
        }
    }]
}

fn pack_tokens_in_mint_token_fragments(token: &InitialToken) -> Result<Vec<Fragment>, Error> {
    let InitialToken {
        token_id,
        policy,
        to,
    } = token;
    let token_id: identifier::TokenIdentifier = token_id.clone().into();
    to.iter()
        .map(|destination| {
            let to = match &destination.address.1 .1 {
                Kind::Account(pk) => pk,
                Kind::Single(pk) => pk,
                _ => return Err(Error::TokenInvalidAddressType),
            };

            let mint_token = certificate::MintToken {
                name: token_id.token_name.clone(),
                policy: policy.clone().into(),
                to: to.clone().into(),
                value: destination.value.into(),
            };

            if mint_token.policy.hash() != token_id.policy_hash {
                return Err(Error::TokenIdentifierMintingPolicyHashMismatch);
            }

            Ok(Fragment::MintToken(Transaction::block0_payload(
                &mint_token,
                &(),
            )))
        })
        .collect::<Result<Vec<_>, _>>()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        crypto::key::KeyPair, interfaces::ARBITRARY_MAX_NUMBER_ENTRIES_PER_INITIAL_FRAGMENT,
    };
    use quickcheck::{Arbitrary, Gen, TestResult};

    impl Arbitrary for Initial {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            let number_entries =
                1 + (usize::arbitrary(g) % ARBITRARY_MAX_NUMBER_ENTRIES_PER_INITIAL_FRAGMENT);
            match u8::arbitrary(g) % 2 {
                0 => Initial::Fund(
                    std::iter::repeat_with(|| Arbitrary::arbitrary(g))
                        .take(number_entries)
                        .collect(),
                ),
                1 => Initial::LegacyFund(
                    std::iter::repeat_with(|| Arbitrary::arbitrary(g))
                        .take(number_entries)
                        .collect(),
                ),
                _ => unreachable!(),
            }
        }
    }

    impl Arbitrary for InitialUTxO {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            InitialUTxO {
                address: Arbitrary::arbitrary(g),
                value: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for LegacyUTxO {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            LegacyUTxO {
                address: Arbitrary::arbitrary(g),
                value: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for Destination {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let kp: KeyPair<chain_crypto::Ed25519> = KeyPair::arbitrary(g);
            let pk: chain_crypto::PublicKey<chain_crypto::Ed25519> =
                kp.identifier().into_public_key();

            let mut address = Address::arbitrary(g);
            address.1 = chain_addr::Address(address.1 .0, chain_addr::Kind::Account(pk));

            Self {
                address,
                value: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for InitialToken {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            Self {
                token_id: Arbitrary::arbitrary(g),
                policy: Arbitrary::arbitrary(g),
                to: vec![Arbitrary::arbitrary(g)],
            }
        }
    }

    quickcheck! {
        fn initial_utxo_serde_human_readable_encode_decode(utxo: InitialUTxO) -> TestResult {
            let s = serde_yaml::to_string(&utxo).unwrap();
            let utxo_dec: InitialUTxO = serde_yaml::from_str(&s).unwrap();

            TestResult::from_bool(utxo == utxo_dec)
        }

        fn legacy_utxo_serde_human_readable_encode_decode(utxo: LegacyUTxO) -> TestResult {
            let s = serde_yaml::to_string(&utxo).unwrap();
            let utxo_dec: LegacyUTxO = serde_yaml::from_str(&s).unwrap();

            TestResult::from_bool(utxo == utxo_dec)
        }

        fn initial_serde_human_readable_encode_decode(initial: Initial) -> TestResult {
            let s = serde_yaml::to_string(&initial).unwrap();
            let initial_dec: Initial = serde_yaml::from_str(&s).unwrap();

            TestResult::from_bool(initial == initial_dec)
        }
    }
}
