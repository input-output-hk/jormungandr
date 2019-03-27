use chain_addr::{AddressReadable, Discrimination};
use chain_core::property::HasMessages as _;
use chain_crypto::{bech32::Bech32, Ed25519Extended, PublicKey};
use chain_impl_mockchain::{
    block::{Block, BlockBuilder, BlockVersion},
    config::{
        entity_from, entity_from_string, entity_to, entity_to_string, Block0Date, ConfigParam,
    },
    fee::LinearFee,
    legacy::{self, OldAddress},
    message::{InitialEnts, Message},
    setting::UpdateProposal,
    transaction,
    value::Value,
};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Serialize, Deserialize)]
pub struct Genesis {
    pub blockchain_configuration: Configuration,

    pub initial_setting: Update,

    pub initial_utxos: Option<Vec<InitialUTxO>>,
    pub legacy_utxos: Option<Vec<LegacyUTxO>>,
}

/// the initial configuration of the blockchain
///
/// * the start date of the block 0;
/// * the discrimination;
/// * ...
///
/// All that is static and does not need to have any update
/// mechanism.
#[derive(Clone, Serialize, Deserialize)]
pub struct Configuration(Vec<(String, String)>);

/// the initial configuration of the blockchain
///
/// This is the data tha may be updated but which needs
/// to have an initial value in the blockchain (or not)
#[derive(Clone, Serialize, Deserialize)]
pub struct Update {
    max_number_of_transactions_per_block: Option<u32>,
    bootstrap_key_slots_percentage: Option<u8>,
    block_version: String,
    bft_leaders: Option<Vec<String>>,
    allow_account_creation: Option<bool>,
    linear_fee: Option<InitialLinearFee>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InitialLinearFee {
    coefficient: u64,
    constant: u64,
    certificate: u64,
}

#[derive(Clone)]
pub struct InitialUTxO {
    pub address: AddressReadable,
    pub value: Value,
}

#[derive(Clone)]
pub struct LegacyUTxO {
    pub address: OldAddress,
    pub value: Value,
}

impl Genesis {
    pub fn from_block<'a>(block: &'a Block) -> Self {
        let mut messages = block.messages();

        let blockchain_configuration = if let Some(Message::Initial(initial)) = messages.next() {
            Configuration::from_message(initial)
        } else {
            panic!("Expecting the second Message of the block 0 to be `Message::Initial`")
        };
        let initial_setting = if let Some(Message::Update(update)) = messages.next() {
            Update::from_message(update)
        } else {
            panic!("Expecting the second Message of the block 0 to be `Message::Update`")
        };

        let mut messages = messages.peekable();

        let initial_utxos = get_initial_utxos(&mut messages);
        let legacy_utxos = get_legacy_utxos(&mut messages);

        Genesis {
            blockchain_configuration: blockchain_configuration,
            initial_setting: initial_setting,
            initial_utxos: if initial_utxos.is_empty() {
                None
            } else {
                Some(initial_utxos)
            },
            legacy_utxos: if legacy_utxos.is_empty() {
                None
            } else {
                Some(legacy_utxos)
            },
        }
    }

    pub fn to_block(&self) -> Block {
        let mut builder = BlockBuilder::new();

        builder.message(self.blockchain_configuration.to_message());
        builder.message(self.initial_setting.clone().to_message());

        builder.messages(
            self.to_initial_messages(
                self.initial_setting
                    .max_number_of_transactions_per_block
                    .unwrap_or(255) as usize,
            ),
        );
        builder.messages(
            self.to_legacy_messages(
                self.initial_setting
                    .max_number_of_transactions_per_block
                    .unwrap_or(255) as usize,
            ),
        );

        builder.make_genesis_block()
    }

    fn to_initial_messages(&self, max_output_per_message: usize) -> Vec<Message> {
        let mut messages = Vec::new();
        if let Some(initial_utxos) = &self.initial_utxos {
            let mut utxo_iter = initial_utxos.iter();

            while let Some(utxo) = utxo_iter.next() {
                let mut outputs = Vec::with_capacity(max_output_per_message);
                outputs.push(transaction::Output {
                    address: utxo.address.to_address(),
                    value: utxo.value,
                });

                while let Some(utxo) = utxo_iter.next() {
                    outputs.push(transaction::Output {
                        address: utxo.address.to_address(),
                        value: utxo.value,
                    });
                    if outputs.len() == max_output_per_message {
                        break;
                    }
                }

                let transaction = transaction::AuthenticatedTransaction {
                    transaction: transaction::Transaction {
                        inputs: Vec::new(),
                        outputs: outputs,
                        extra: transaction::NoExtra,
                    },
                    witnesses: Vec::new(),
                };
                messages.push(Message::Transaction(transaction));
            }
        }

        messages
    }
    fn to_legacy_messages(&self, max_output_per_message: usize) -> Vec<Message> {
        let mut messages = Vec::new();
        if let Some(legacy_utxos) = &self.legacy_utxos {
            let mut utxo_iter = legacy_utxos.iter();

            while let Some(utxo) = utxo_iter.next() {
                let mut outputs = Vec::with_capacity(max_output_per_message);
                outputs.push((utxo.address.clone(), utxo.value));

                while let Some(utxo) = utxo_iter.next() {
                    outputs.push((utxo.address.clone(), utxo.value));
                    if outputs.len() == max_output_per_message {
                        break;
                    }
                }

                let declaration = legacy::UtxoDeclaration { addrs: outputs };

                messages.push(Message::OldUtxoDeclaration(declaration));
            }
        }

        messages
    }
}

fn get_initial_utxos<'a>(
    messages: &mut std::iter::Peekable<
        std::boxed::Box<
            (dyn std::iter::Iterator<Item = &'a chain_impl_mockchain::message::Message> + 'a),
        >,
    >,
) -> Vec<InitialUTxO> {
    let mut vec = Vec::new();

    while let Some(Message::Transaction(transaction)) = messages.peek() {
        messages.next();
        if !transaction.transaction.inputs.is_empty() {
            panic!("Expected every transaction to not have any inputs");
        }

        for output in transaction.transaction.outputs.iter() {
            let initial_utxo = InitialUTxO {
                address: AddressReadable::from_address(&output.address),
                value: output.value,
            };

            vec.push(initial_utxo);
        }
    }

    vec
}
fn get_legacy_utxos<'a>(
    messages: &mut std::iter::Peekable<
        std::boxed::Box<
            (dyn std::iter::Iterator<Item = &'a chain_impl_mockchain::message::Message> + 'a),
        >,
    >,
) -> Vec<LegacyUTxO> {
    let mut vec = Vec::new();

    while let Some(Message::OldUtxoDeclaration(old_decls)) = messages.peek() {
        messages.next();
        for (address, value) in old_decls.addrs.iter() {
            let legacy_utxo = LegacyUTxO {
                address: address.clone(),
                value: value.clone(),
            };

            vec.push(legacy_utxo);
        }
    }

    vec
}

impl Update {
    pub fn to_message(self) -> Message {
        let update = UpdateProposal {
            max_number_of_transactions_per_block: self.max_number_of_transactions_per_block,
            bootstrap_key_slots_percentage: self.bootstrap_key_slots_percentage,
            block_version: Some({
                let v = self.block_version.parse::<u16>().unwrap();
                BlockVersion::new(v)
            }),
            bft_leaders: self.bft_leaders.clone().map(|leaders| {
                leaders
                    .iter()
                    .map(|leader| {
                        <PublicKey<Ed25519Extended> as Bech32>::try_from_bech32_str(&leader)
                            .unwrap()
                            .into()
                    })
                    .collect()
            }),
            allow_account_creation: self.allow_account_creation,
            linear_fees: self.linear_fee.map(|linear_fee| LinearFee {
                constant: linear_fee.constant,
                coefficient: linear_fee.coefficient,
                certificate: linear_fee.certificate,
            }),
        };
        Message::Update(update)
    }
    pub fn from_message(update_proposal: &UpdateProposal) -> Self {
        Update {
            max_number_of_transactions_per_block: update_proposal
                .max_number_of_transactions_per_block,
            bootstrap_key_slots_percentage: update_proposal.bootstrap_key_slots_percentage,
            block_version: update_proposal
                .block_version
                .map(|bv| format!("{}", bv.as_u16()))
                .unwrap_or("1".to_owned()),
            bft_leaders: update_proposal.bft_leaders.clone().map(|leaders| {
                leaders
                    .iter()
                    .map(|leader| leader.as_public_key().to_bech32_str())
                    .collect()
            }),
            allow_account_creation: update_proposal.allow_account_creation,
            linear_fee: update_proposal
                .linear_fees
                .map(|linear_fee| InitialLinearFee {
                    constant: linear_fee.constant,
                    coefficient: linear_fee.coefficient,
                    certificate: linear_fee.certificate,
                }),
        }
    }
}

impl Configuration {
    pub fn from_message(initial_ents: &InitialEnts) -> Self {
        let mut data = Vec::with_capacity(initial_ents.iter().len());

        for (t, v) in initial_ents.iter() {
            match t {
                &<Block0Date as ConfigParam>::TAG => {
                    let t = entity_from::<Block0Date>(*t, v).expect("Failed to parse block0-date");
                    let (k, v) = entity_to_string(&t);
                    data.push((k.to_owned(), v));
                }
                &<Discrimination as ConfigParam>::TAG => {
                    let t = entity_from::<Discrimination>(*t, v)
                        .expect("Failed to parse discrimination");
                    let (k, v) = entity_to_string(&t);
                    data.push((k.to_owned(), v));
                }
                _ => panic!(),
            }
        }

        Configuration(data)
    }

    pub fn to_message(&self) -> Message {
        let mut initial = InitialEnts::new();
        for (t, v) in self.0.iter() {
            match t.as_str() {
                <Block0Date as ConfigParam>::NAME => {
                    let t = entity_from_string::<Block0Date>(t, v).unwrap();
                    initial.push(entity_to(&t));
                }
                <Discrimination as ConfigParam>::NAME => {
                    let t = match entity_from_string::<Discrimination>(t, v) {
                        Err(err) => panic!("{:?}, expected values (`test' or `production')", err),
                        Ok(v) => v,
                    };
                    initial.push(entity_to(&t));
                }
                s => panic!(
                    "Unknown tag: {} (supported: {}, {})",
                    s,
                    <Block0Date as ConfigParam>::NAME,
                    <Discrimination as ConfigParam>::NAME,
                ),
            }
        }
        Message::Initial(initial)
    }
}

impl serde::ser::Serialize for LegacyUTxO {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("LegacyUTxO", 2)?;
        state.serialize_field("address", &self.address)?;
        state.serialize_field("value", &self.value.0)?;
        state.end()
    }
}

impl serde::ser::Serialize for InitialUTxO {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("InitialUTxO", 2)?;
        state.serialize_field("address", self.address.as_string())?;
        state.serialize_field("value", &self.value.0)?;
        state.end()
    }
}

impl<'de> serde::de::Deserialize<'de> for LegacyUTxO {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
        const FIELDS: &'static [&'static str] = &["address", "value"];

        enum Field {
            Address,
            Value,
        };

        struct InitialUTxOVisitor;

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`address` or `value`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "address" => Ok(Field::Address),
                            "value" => Ok(Field::Value),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                impl<'de> Visitor<'de> for InitialUTxOVisitor {
                    type Value = LegacyUTxO;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("struct Duration")
                    }

                    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
                    where
                        V: SeqAccess<'de>,
                    {
                        let address: OldAddress = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                        let value = seq
                            .next_element()?
                            .map(Value)
                            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                        Ok(LegacyUTxO { address, value })
                    }

                    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
                    where
                        V: MapAccess<'de>,
                    {
                        let mut address = None;
                        let mut value = None;
                        while let Some(key) = map.next_key()? {
                            match key {
                                Field::Address => {
                                    if address.is_some() {
                                        return Err(de::Error::duplicate_field("address"));
                                    }
                                    address = Some({ map.next_value::<OldAddress>()? });
                                }
                                Field::Value => {
                                    if value.is_some() {
                                        return Err(de::Error::duplicate_field("value"));
                                    }
                                    value = Some(map.next_value().map(Value)?);
                                }
                            }
                        }
                        let address = address.ok_or_else(|| de::Error::missing_field("address"))?;
                        let value = value.ok_or_else(|| de::Error::missing_field("value"))?;
                        Ok(LegacyUTxO { address, value })
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }
        deserializer.deserialize_struct("InitialUTxO", FIELDS, InitialUTxOVisitor)
    }
}

impl<'de> serde::de::Deserialize<'de> for InitialUTxO {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
        const FIELDS: &'static [&'static str] = &["address", "value"];

        enum Field {
            Address,
            Value,
        };

        struct InitialUTxOVisitor;

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`address` or `value`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "address" => Ok(Field::Address),
                            "value" => Ok(Field::Value),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                impl<'de> Visitor<'de> for InitialUTxOVisitor {
                    type Value = InitialUTxO;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("struct Duration")
                    }

                    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
                    where
                        V: SeqAccess<'de>,
                    {
                        let address = seq
                            .next_element()?
                            .map(|s: String| AddressReadable::from_string(&s))
                            .ok_or_else(|| de::Error::invalid_length(0, &self))?
                            .map_err(de::Error::custom)?;
                        let value = seq
                            .next_element()?
                            .map(Value)
                            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                        Ok(InitialUTxO { address, value })
                    }

                    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
                    where
                        V: MapAccess<'de>,
                    {
                        let mut address = None;
                        let mut value = None;
                        while let Some(key) = map.next_key()? {
                            match key {
                                Field::Address => {
                                    if address.is_some() {
                                        return Err(de::Error::duplicate_field("address"));
                                    }
                                    address = Some({
                                        let value = map.next_value::<String>()?;
                                        AddressReadable::from_string(&value)
                                            .map_err(de::Error::custom)?
                                    });
                                }
                                Field::Value => {
                                    if value.is_some() {
                                        return Err(de::Error::duplicate_field("value"));
                                    }
                                    value = Some(map.next_value().map(Value)?);
                                }
                            }
                        }
                        let address = address.ok_or_else(|| de::Error::missing_field("address"))?;
                        let value = value.ok_or_else(|| de::Error::missing_field("value"))?;
                        Ok(InitialUTxO { address, value })
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }
        deserializer.deserialize_struct("InitialUTxO", FIELDS, InitialUTxOVisitor)
    }
}
