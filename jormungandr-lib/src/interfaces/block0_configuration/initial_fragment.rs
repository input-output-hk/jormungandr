use crate::interfaces::{Address, Certificate, OldAddress, Value};
use chain_addr;
use chain_impl_mockchain::{
    certificate,
    fragment::Fragment,
    legacy::UtxoDeclaration,
    transaction::{AuthenticatedTransaction, NoExtra, Output, Transaction},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Initial {
    Fund(Vec<InitialUTxO>),
    Cert(Certificate),
    LegacyFund(Vec<LegacyUTxO>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialUTxO {
    pub address: Address,
    pub value: Value,
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
            Fragment::Transaction(tx) => try_extend_inits_with_tx(&mut inits, tx)?,
            Fragment::OldUtxoDeclaration(utxo) => extend_inits_with_legacy_utxo(&mut inits, utxo),
            Fragment::PoolRegistration(tx) => {
                let cert = certificate::Certificate::PoolRegistration(tx.transaction.extra.clone());
                inits.push(Initial::Cert(Certificate(cert)))
            }
            Fragment::StakeDelegation(tx) => {
                let cert = certificate::Certificate::StakeDelegation(tx.transaction.extra.clone());
                inits.push(Initial::Cert(Certificate(cert)))
            }
            _ => return Err(Error::Block0MessageUnexpected),
        }
    }
    Ok(inits)
}

fn try_extend_inits_with_tx(
    initials: &mut Vec<Initial>,
    tx: &AuthenticatedTransaction<chain_addr::Address, NoExtra>,
) -> Result<(), Error> {
    if !tx.transaction.inputs.is_empty() {
        return Err(Error::InitUtxoHasInput);
    }
    let inits_iter = tx.transaction.outputs.iter().map(|output| InitialUTxO {
        address: output.address.clone().into(),
        value: output.value.into(),
    });
    initials.push(Initial::Fund(inits_iter.collect()));
    Ok(())
}

fn extend_inits_with_legacy_utxo(initials: &mut Vec<Initial>, utxo_decl: &UtxoDeclaration) {
    let inits_iter = utxo_decl.addrs.iter().map(|(address, value)| LegacyUTxO {
        address: address.clone().into(),
        value: value.clone().into(),
    });
    initials.push(Initial::LegacyFund(inits_iter.collect()))
}

impl<'a> From<&'a Initial> for Fragment {
    fn from(initial: &'a Initial) -> Fragment {
        match initial {
            Initial::Fund(utxo) => pack_utxo_in_message(&utxo),
            Initial::Cert(cert) => pack_certificate_in_message(&cert),
            Initial::LegacyFund(utxo) => pack_legacy_utxo_in_message(&utxo),
        }
    }
}

fn pack_utxo_in_message(v: &[InitialUTxO]) -> Fragment {
    let outputs = v
        .iter()
        .map(|utxo| Output {
            address: utxo.address.clone().into(),
            value: utxo.value.into(),
        })
        .collect();

    Fragment::Transaction(AuthenticatedTransaction {
        transaction: Transaction {
            inputs: vec![],
            outputs: outputs,
            extra: NoExtra,
        },
        witnesses: vec![],
    })
}

fn pack_legacy_utxo_in_message(v: &[LegacyUTxO]) -> Fragment {
    let addrs = v
        .iter()
        .map(|utxo| (utxo.address.clone().into(), utxo.value.into()))
        .collect();
    Fragment::OldUtxoDeclaration(UtxoDeclaration { addrs: addrs })
}

fn empty_auth_tx<Payload: Clone>(payload: &Payload) -> AuthenticatedTransaction<chain_addr::Address, Payload> {
    AuthenticatedTransaction {
        transaction: Transaction {
            inputs: vec![],
            outputs: vec![],
            extra: payload.clone(),
        },
        witnesses: vec![],
    }
}

fn pack_certificate_in_message(cert: &Certificate) -> Fragment {
    match &cert.0 {
        certificate::Certificate::StakeDelegation(c) =>
            Fragment::StakeDelegation(empty_auth_tx(c)),
        certificate::Certificate::OwnerStakeDelegation(c) =>
            Fragment::OwnerStakeDelegation(empty_auth_tx(c)),
        certificate::Certificate::PoolRegistration(c) =>
            Fragment::PoolRegistration(empty_auth_tx(c)),
        certificate::Certificate::PoolManagement(c) =>
            Fragment::PoolManagement(empty_auth_tx(c)),
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
                usize::arbitrary(g) % ARBITRARY_MAX_NUMBER_ENTRIES_PER_INITIAL_FRAGMENT;
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
