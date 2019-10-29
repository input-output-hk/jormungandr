use crate::interfaces::{Address, OldAddress, SignedCertificate, Value};
use chain_addr;
use chain_impl_mockchain::{
    certificate,
    fragment::Fragment,
    legacy::UtxoDeclaration,
    transaction::{NoExtra, Output, Payload, Transaction, TransactionSlice, TxBuilder},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Initial {
    Fund(Vec<InitialUTxO>),
    Cert(SignedCertificate),
    LegacyFund(Vec<LegacyUTxO>),
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

custom_error! {pub Error
    FirstBlock0MessageNotInit = "first message of block 0 is not initial",
    Block0MessageUnexpected  = "non-first message of block 0 has unexpected type",
    InitUtxoHasInput = "initial UTXO has input",
}

pub fn try_initials_vec_from_messages<'a>(
    messages: impl Iterator<Item = &'a Fragment>,
) -> Result<Vec<Initial>, Error> {
    let mut inits = Vec::new();
    for message in messages {
        match message {
            Fragment::Transaction(tx) => try_extend_inits_with_tx(&mut inits, &tx.as_slice())?,
            Fragment::OldUtxoDeclaration(utxo) => extend_inits_with_legacy_utxo(&mut inits, utxo),
            Fragment::PoolRegistration(tx) => {
                let tx = tx.as_slice();
                let cert = tx.payload().into_payload();
                let auth = tx.payload_auth().into_payload_auth();
                let cert = certificate::SignedCertificate::PoolRegistration(cert, auth);
                inits.push(Initial::Cert(cert.into()))
            }
            Fragment::StakeDelegation(tx) => {
                let tx = tx.as_slice();
                let cert = tx.payload().into_payload();
                let auth = tx.payload_auth().into_payload_auth();
                let cert = certificate::SignedCertificate::StakeDelegation(cert, auth);
                inits.push(Initial::Cert(cert.into()))
            }
            _ => return Err(Error::Block0MessageUnexpected),
        }
    }
    Ok(inits)
}

fn try_extend_inits_with_tx<'a>(
    initials: &mut Vec<Initial>,
    tx: &TransactionSlice<'a, NoExtra>,
) -> Result<(), Error> {
    if tx.nb_inputs() != 0 {
        return Err(Error::InitUtxoHasInput);
    }
    let inits_iter = tx.outputs().iter().map(|output| InitialUTxO {
        address: output.address.clone().into(),
        value: output.value.into(),
    });
    initials.push(Initial::Fund(inits_iter.collect()));
    Ok(())
}

fn extend_inits_with_legacy_utxo(initials: &mut Vec<Initial>, utxo_decl: &UtxoDeclaration) {
    if utxo_decl.addrs.len() == 0 {
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
        value: value.clone().into(),
    });
    let inits: Vec<_> = inits_iter.collect();
    initials.push(Initial::LegacyFund(inits))
}

impl<'a> From<&'a Initial> for Fragment {
    fn from(initial: &'a Initial) -> Fragment {
        match initial {
            Initial::Fund(utxo) => pack_utxo_in_message(&utxo),
            Initial::Cert(cert) => pack_certificate_in_empty_tx_fragment(&cert),
            Initial::LegacyFund(utxo) => pack_legacy_utxo_in_message(&utxo),
        }
    }
}

fn pack_utxo_in_message(v: &[InitialUTxO]) -> Fragment {
    let outputs: Vec<_> = v.iter().map(|utxo| utxo.to_output()).collect();

    if outputs.len() == 0 {
        panic!("cannot create a singular transaction fragment with 0 output")
    }
    if outputs.len() >= 255 {
        panic!("cannot create a singular transaction fragment with more than 254 outputs ({} requested). spread outputs to another fragment", outputs.len())
    }

    let tx = TxBuilder::new()
        .set_nopayload()
        .set_ios(&[], &outputs[..])
        .set_witnesses(&[])
        .set_payload_auth(&());
    Fragment::Transaction(tx)
}

fn pack_legacy_utxo_in_message(v: &[LegacyUTxO]) -> Fragment {
    if v.len() == 0 {
        panic!("cannot create a singular legacy declaration fragment with 0 declaration")
    }
    if v.len() >= 255 {
        panic!("cannot create a singular legacy declaration fragment with more than 254 declarations ({} requested). spread declarations to another fragment", v.len())
    }
    let addrs = v
        .iter()
        .map(|utxo| (utxo.address.clone().into(), utxo.value.into()))
        .collect();
    Fragment::OldUtxoDeclaration(UtxoDeclaration { addrs: addrs })
}

fn empty_auth_tx<P: Payload>(payload: &P, payload_auth: &P::Auth) -> Transaction<P> {
    Transaction::block0_payload(payload, payload_auth)
}

fn pack_certificate_in_empty_tx_fragment(cert: &SignedCertificate) -> Fragment {
    match &cert.0 {
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
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::interfaces::ARBITRARY_MAX_NUMBER_ENTRIES_PER_INITIAL_FRAGMENT;
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
