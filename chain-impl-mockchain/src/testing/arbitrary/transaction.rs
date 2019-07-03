use crate::ledger::Ledger;
use crate::testing::arbitrary::transaction::rand::prelude::SliceRandom;
use crate::transaction::{Input, Output};
use crate::utxo::{self, Entry};
use crate::{
    testing::address::{AddressData, AddressDataValue},
    value::*,
};
use chain_addr::Address;
use quickcheck::{Arbitrary, Gen};
use std::iter;

extern crate rand;
use super::ArbitraryAddressDataVec;
use rand::Rng;

#[derive(Clone, Debug)]
pub struct AribtraryValidTransactionData {
    pub addresses: Vec<AddressDataValue>,
    input_addresses: Vec<AddressDataValue>,
    output_addresses: Vec<AddressDataValue>,
}

impl Arbitrary for AribtraryValidTransactionData {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        AribtraryValidTransactionDataBuilder::new(ArbitraryAddressDataVec::arbitrary(gen).0).build()
    }
}

pub struct AribtraryValidTransactionDataBuilder {
    source: Vec<AddressData>,
}

impl AribtraryValidTransactionDataBuilder {
    pub fn new(source: Vec<AddressData>) -> Self {
        AribtraryValidTransactionDataBuilder { source: source }
    }

    pub fn build(&self) -> AribtraryValidTransactionData {
        let values: Vec<Value> = Self::generate_random_values()
            .take(self.source.len())
            .collect();

        let addresses_values: Vec<AddressDataValue> =
            Self::zip_addresses_and_values(&self.source, values);
        let input_addresses_values = Self::choose_random_subset(&addresses_values);
        let total_input_value = Self::sum_up_funds(
            input_addresses_values
                .iter()
                .cloned()
                .map(|x| x.value)
                .collect(),
        );
        let output_addresses_values =
            Self::choose_random_output_subset(&addresses_values, total_input_value);
        AribtraryValidTransactionData::new(
            addresses_values,
            input_addresses_values,
            output_addresses_values,
        )
    }

    fn generate_random_values() -> impl Iterator<Item = Value> {
        iter::from_fn(|| Some(Self::generate_single_random_value(1, 200)))
    }

    fn generate_single_random_value(lower_bound: u64, upper_bound: u64) -> Value {
        let random_number: u64 = rand::thread_rng().gen_range(lower_bound, upper_bound);
        Value(random_number)
    }

    fn zip_addresses_and_values(
        addresses: &Vec<AddressData>,
        values: Vec<Value>,
    ) -> Vec<AddressDataValue> {
        addresses
            .iter()
            .cloned()
            .zip(values.iter())
            .map(|(x, y)| AddressDataValue::new(x, *y))
            .collect()
    }

    fn sum_up_funds(values: Vec<Value>) -> u64 {
        values.iter().map(|x| x.0).sum()
    }

    fn choose_random_subset(source: &Vec<AddressDataValue>) -> Vec<AddressDataValue> {
        let mut rng = rand::thread_rng();
        let lower_bound = 1;
        let upper_bound = source.len();
        if upper_bound <= lower_bound {
            return source.iter().cloned().collect();
        }
        let random_length: usize = rng.gen_range(lower_bound, upper_bound);
        let source = source
            .choose_multiple(&mut rng, random_length)
            .cloned()
            .collect();
        println!("{:?}", source);
        source
    }

    fn choose_random_output_subset(
        source: &Vec<AddressDataValue>,
        total_input_funds: u64,
    ) -> Vec<AddressDataValue> {
        let mut outputs: Vec<AddressData>;
        let mut funds_per_output: u64;
        loop {
            outputs = Self::choose_random_subset(source)
                .iter()
                .cloned()
                .map(|x| x.address_data)
                .collect();
            funds_per_output = total_input_funds / outputs.len() as u64;
            if funds_per_output > 0 {
                break;
            }
        }
        let output_address_len = outputs.len() as u64;
        let remainder = total_input_funds - (output_address_len * funds_per_output);
        Self::distribute_values_for_outputs(outputs, funds_per_output, remainder)
    }

    fn distribute_values_for_outputs(
        outputs: Vec<AddressData>,
        funds_per_output: u64,
        remainder: u64,
    ) -> Vec<AddressDataValue> {
        let mut outputs: Vec<AddressDataValue> = outputs
            .iter()
            .cloned()
            .zip(iter::from_fn(|| Some(Value(funds_per_output))))
            .map(|(x, y)| AddressDataValue::new(x, y))
            .collect();
        outputs[0].value = Value(funds_per_output + remainder);
        outputs
    }
}

impl AribtraryValidTransactionData {
    pub fn new(
        addresses: Vec<AddressDataValue>,
        input_addresses_values: Vec<AddressDataValue>,
        output_addresses_values: Vec<AddressDataValue>,
    ) -> Self {
        AribtraryValidTransactionData {
            addresses: addresses,
            input_addresses: input_addresses_values,
            output_addresses: output_addresses_values,
        }
    }

    fn find_utxo_for_address<'a>(
        address_data: &AddressData,
        utxos: &mut utxo::Iter<'a, Address>,
    ) -> Option<Entry<'a, Address>> {
        utxos.find(|x| x.output.address == address_data.address)
    }

    fn make_single_input(
        &self,
        address_data_value: AddressDataValue,
        mut utxos: &mut utxo::Iter<'_, Address>,
    ) -> Input {
        let utxo_option = Self::find_utxo_for_address(&address_data_value.address_data, &mut utxos);
        address_data_value.make_input(utxo_option)
    }

    pub fn make_inputs(&mut self, ledger: &Ledger) -> Vec<Input> {
        self.input_addresses
            .iter()
            .cloned()
            .map(|x| self.make_single_input(x, &mut ledger.utxos()))
            .collect()
    }

    pub fn make_outputs_from_all_addresses(&self) -> Vec<Output<Address>> {
        self.addresses.iter().map(|x| x.make_output()).collect()
    }

    pub fn make_outputs(&mut self) -> Vec<Output<Address>> {
        self.output_addresses
            .iter()
            .map(|x| x.make_output())
            .collect()
    }

    pub fn input_addresses(&mut self) -> Vec<AddressData> {
        self.input_addresses
            .iter()
            .cloned()
            .map(|x| x.address_data)
            .collect()
    }
}
