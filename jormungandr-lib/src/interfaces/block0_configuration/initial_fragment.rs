use crate::interfaces::{Address, Certificate, OldAddress, Value};
use chain_impl_mockchain::{
    certificate,
    legacy::UtxoDeclaration,
    message::Message,
    transaction::{AuthenticatedTransaction, NoExtra, Output, Transaction},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    messages: impl Iterator<Item = &'a Message>,
) -> Result<Vec<Initial>, Error> {
    let mut inits = Vec::new();
    for message in messages {
        match message {
            Message::Transaction(tx) => try_extend_inits_with_tx(&mut inits, tx)?,
            Message::Certificate(tx) => extend_inits_with_cert(&mut inits, tx),
            Message::OldUtxoDeclaration(utxo) => extend_inits_with_legacy_utxo(&mut inits, utxo),
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

fn extend_inits_with_cert(
    initials: &mut Vec<Initial>,
    tx: &AuthenticatedTransaction<chain_addr::Address, certificate::Certificate>,
) {
    let cert = Certificate::from(tx.transaction.extra.clone());
    initials.push(Initial::Cert(cert))
}

fn extend_inits_with_legacy_utxo(initials: &mut Vec<Initial>, utxo_decl: &UtxoDeclaration) {
    let inits_iter = utxo_decl.addrs.iter().map(|(address, value)| LegacyUTxO {
        address: address.clone().into(),
        value: value.clone().into(),
    });
    initials.push(Initial::LegacyFund(inits_iter.collect()))
}

impl<'a> From<&'a Initial> for Message {
    fn from(initial: &'a Initial) -> Message {
        match initial {
            Initial::Fund(utxo) => pack_utxo_in_message(&utxo),
            Initial::Cert(cert) => cert.into(),
            Initial::LegacyFund(utxo) => pack_legacy_utxo_in_message(&utxo),
        }
    }
}

fn pack_utxo_in_message(v: &[InitialUTxO]) -> Message {
    let outputs = v
        .iter()
        .map(|utxo| Output {
            address: utxo.address.clone().into(),
            value: utxo.value.into(),
        })
        .collect();

    Message::Transaction(AuthenticatedTransaction {
        transaction: Transaction {
            inputs: vec![],
            outputs: outputs,
            extra: NoExtra,
        },
        witnesses: vec![],
    })
}

fn pack_legacy_utxo_in_message(v: &[LegacyUTxO]) -> Message {
    let addrs = v
        .iter()
        .map(|utxo| (utxo.address.clone().into(), utxo.value.into()))
        .collect();
    Message::OldUtxoDeclaration(UtxoDeclaration { addrs: addrs })
}

impl<'a> From<&'a Certificate> for Message {
    fn from(utxo: &'a Certificate) -> Message {
        Message::Certificate(AuthenticatedTransaction {
            transaction: Transaction {
                inputs: vec![],
                outputs: vec![],
                extra: utxo.clone().into(),
            },
            witnesses: vec![],
        })
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
