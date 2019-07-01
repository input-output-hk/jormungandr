use crate::interfaces::{Address, OldAddress, Value};
use chain_impl_mockchain::{
    certificate,
    legacy::UtxoDeclaration,
    message::Message,
    transaction::{AuthenticatedTransaction, NoExtra, Output, Transaction},
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Initial {
    Fund(Vec<InitialUTxO>),
    Cert(Certificate),
    LegacyFund(Vec<LegacyUTxO>),
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialUTxO {
    pub address: Address,
    pub value: Value,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyUTxO {
    pub address: OldAddress,
    pub value: Value,
}

#[derive(Clone, Debug)]
pub struct Certificate(certificate::Certificate);

/* ------------------- Serde ----------------------------------------------- */

impl Serialize for Certificate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use bech32::{Bech32, ToBase32 as _};
        use chain_core::property::Serialize as _;
        use serde::ser::Error as _;

        let bytes = self.0.serialize_as_vec().map_err(S::Error::custom)?;
        let bech32 =
            Bech32::new("cert".to_string(), bytes.to_base32()).map_err(S::Error::custom)?;

        format!("{}", bech32).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Certificate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use bech32::{Bech32, FromBase32 as _};
        use chain_core::mempack::{ReadBuf, Readable as _};
        use serde::de::Error as _;

        let bech32_str = String::deserialize(deserializer)?;
        let bech32: Bech32 = bech32_str.parse().map_err(D::Error::custom)?;
        if bech32.hrp() != "cert" {
            return Err(D::Error::custom(format!(
                "Expecting certificate in bech32, with HRP 'cert'"
            )));
        }
        let bytes: Vec<u8> = Vec::from_base32(bech32.data()).map_err(D::Error::custom)?;
        let mut buf = ReadBuf::from(&bytes);
        certificate::Certificate::read(&mut buf)
            .map_err(D::Error::custom)
            .map(Certificate)
    }
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
    let cert = Certificate(tx.transaction.extra.clone());
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
                extra: utxo.0.clone(),
            },
            witnesses: vec![],
        })
    }
}
