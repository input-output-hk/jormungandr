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
    Fund(InitialUTxO),
    Cert(Certificate),
    LegacyFund(LegacyUTxO),
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
    let inits_iter = tx
        .transaction
        .outputs
        .iter()
        .map(|output| InitialUTxO {
            address: output.address.clone().into(),
            value: output.value.into(),
        })
        .map(Initial::Fund);
    initials.extend(inits_iter);
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
    let inits_iter = utxo_decl
        .addrs
        .iter()
        .map(|(address, value)| LegacyUTxO {
            address: address.clone().into(),
            value: value.clone().into(),
        })
        .map(Initial::LegacyFund);
    initials.extend(inits_iter)
}

impl<'a> From<&'a Initial> for Message {
    fn from(initial: &'a Initial) -> Message {
        match initial {
            Initial::Fund(utxo) => utxo.into(),
            Initial::Cert(cert) => cert.into(),
            Initial::LegacyFund(utxo) => utxo.into(),
        }
    }
}

impl<'a> From<&'a InitialUTxO> for Message {
    fn from(utxo: &'a InitialUTxO) -> Message {
        Message::Transaction(AuthenticatedTransaction {
            transaction: Transaction {
                inputs: vec![],
                outputs: vec![Output {
                    address: utxo.address.clone().into(),
                    value: utxo.value.into(),
                }],
                extra: NoExtra,
            },
            witnesses: vec![],
        })
    }
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

impl<'a> From<&'a LegacyUTxO> for Message {
    fn from(utxo: &'a LegacyUTxO) -> Message {
        Message::OldUtxoDeclaration(UtxoDeclaration {
            addrs: vec![(utxo.address.clone().into(), utxo.value.into())],
        })
    }
}

/*

pub fn documented_example(now: std::time::SystemTime) -> String {
    let secs = now
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let sk: SecretKey<Ed25519Extended> = SecretKey::generate(&mut ChaChaRng::from_seed([0; 32]));
    let pk: PublicKey<Ed25519> = sk.to_public();
    let leader_1: KeyPair<Ed25519> = KeyPair::generate(&mut ChaChaRng::from_seed([1; 32]));
    let leader_2: KeyPair<Ed25519> = KeyPair::generate(&mut ChaChaRng::from_seed([2; 32]));

    let initial_funds_address = Address(Discrimination::Test, Kind::Single(pk));
    let initial_funds_address = AddressReadable::from_address(&initial_funds_address).to_string();
    let leader_1_pk = leader_1.public_key().to_bech32_str();
    let leader_2_pk = leader_2.public_key().to_bech32_str();
    format!(
        include_str!("DOCUMENTED_EXAMPLE.yaml"),
        now = secs,
        leader_1 = leader_1_pk,
        leader_2 = leader_2_pk,
        initial_funds_address = initial_funds_address
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_yaml;

    #[test]
    fn conversion_to_and_from_message_preserves_data() {
        let sk: SecretKey<Ed25519Extended> =
            SecretKey::generate(&mut ChaChaRng::from_seed([0; 32]));
        let pk: PublicKey<Ed25519> = sk.to_public();

        let leader_1: KeyPair<Ed25519> = KeyPair::generate(&mut ChaChaRng::from_seed([1; 32]));
        let leader_2: KeyPair<Ed25519> = KeyPair::generate(&mut ChaChaRng::from_seed([2; 32]));

        let initial_funds_address = Address(Discrimination::Test, Kind::Single(pk));
        let initial_funds_address =
            AddressReadable::from_address(&initial_funds_address).to_string();

        let leader_1_pk = leader_1.public_key().to_bech32_str();
        let leader_2_pk = leader_2.public_key().to_bech32_str();

        let genesis_yaml = format!(r#"
---
blockchain_configuration:
  block0_date: 123456789
  discrimination: test
  block0_consensus: bft
  slots_per_epoch: 5
  slot_duration: 15
  epoch_stability_depth: 10
  consensus_leader_ids:
    - {}
    - {}
  consensus_genesis_praos_active_slot_coeff: "0.444"
  max_number_of_transactions_per_block: 255
  bft_slots_ratio: "0.222"
  linear_fees:
    coefficient: 1
    constant: 2
    certificate: 4
  kes_update_speed: 43200
initial:
  - cert: cert1qgqqqqqqqqqqqqqqqqqqq0p5avfqqmgurpe7s9k7933q0wj420jl5xqvx8lywcu5jcr7fwqa9qmdn93q4nm7c4fsay3mzeqgq3c0slnut9kns08yn2qn80famup7nvgtfuyszqzqrd4lxlt5ylplfu76p8f6ks0ggprzatp2c8rn6ev3hn9dgr38tzful4h0udlwa0536vyrrug7af9ujmrr869afs0yw9gj5x7z24l8sps3zzcmv
  - fund:
      address: {}
      value: 10000"#, leader_1_pk, leader_2_pk, initial_funds_address);
        let genesis: Genesis =
            serde_yaml::from_str(genesis_yaml.as_str()).expect("Failed to deserialize YAML");

        let block = genesis.to_block();
        let new_genesis = Genesis::from_block(&block).expect("Failed to build genesis");

        let new_genesis_yaml =
            serde_yaml::to_string(&new_genesis).expect("Failed to serialize YAML");
        assert_eq!(
            genesis_yaml.trim(),
            new_genesis_yaml,
            "\nGenesis YAML has changed after conversions:\n{}\n",
            new_genesis_yaml
        );
    }
}

*/
