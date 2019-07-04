use super::ArbitraryAddressDataVec;
use crate::ledger::Ledger;
use crate::transaction::{Input, Output};
use crate::utxo::{self, Entry};
use crate::{
    testing::address::{AddressData, AddressDataValue},
    testing::arbitrary::AverageValue,
    value::*,
};
use chain_addr::Address;
use quickcheck::{Arbitrary, Gen};
use std::cmp;
use std::collections::HashSet;
use std::iter;

#[derive(Clone, Debug)]
pub struct ArbitraryValidTransactionData {
    pub addresses: Vec<AddressDataValue>,
    input_addresses: Vec<AddressDataValue>,
    output_addresses: Vec<AddressDataValue>,
}

impl Arbitrary for ArbitraryValidTransactionData {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        use ArbitraryValidTransactionData as tx_data;
        let source = ArbitraryAddressDataVec::arbitrary(gen);
        let values: Vec<Value> = iter::from_fn(|| Some(AverageValue::arbitrary(gen)))
            .map(|x| x.into())
            .take(source.0.len())
            .collect();
        let addresses_values: Vec<AddressDataValue> =
            tx_data::zip_addresses_and_values(&source.0, values);
        let input_addresses_values = tx_data::choose_random_subset(&addresses_values, gen);
        let total_input_value = input_addresses_values
            .iter()
            .cloned()
            .map(|x| x.value.0)
            .sum();
        let output_addresses_values =
            tx_data::choose_random_output_subset(&addresses_values, total_input_value, gen);
        ArbitraryValidTransactionData::new(
            addresses_values,
            input_addresses_values,
            output_addresses_values,
        )
    }
}

impl ArbitraryValidTransactionData {
    pub fn new(
        addresses: Vec<AddressDataValue>,
        input_addresses_values: Vec<AddressDataValue>,
        output_addresses_values: Vec<AddressDataValue>,
    ) -> Self {
        ArbitraryValidTransactionData {
            addresses: addresses,
            input_addresses: input_addresses_values,
            output_addresses: output_addresses_values,
        }
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

    fn choose_random_subset<G: Gen>(
        source: &Vec<AddressDataValue>,
        gen: &mut G,
    ) -> Vec<AddressDataValue> {
        let lower_bound = 1;
        let upper_bound = source.len();
        let mut arbitrary_indexes = HashSet::new();

        // set limit between lower_bound and upper_bound
        let random_length = cmp::max(usize::arbitrary(gen) % upper_bound, lower_bound);

        // choose arbitrary non-repertive indexes
        while arbitrary_indexes.len() < random_length {
            let random_number = usize::arbitrary(gen) % upper_bound;
            arbitrary_indexes.insert(random_number);
        }

        // create sub collecion from arbitrary indexes
        source
            .iter()
            .cloned()
            .enumerate()
            .filter(|(i, _)| arbitrary_indexes.contains(i))
            .map(|(_, e)| e)
            .collect()
    }

    fn choose_random_output_subset<G: Gen>(
        source: &Vec<AddressDataValue>,
        total_input_funds: u64,
        gen: &mut G,
    ) -> Vec<AddressDataValue> {
        let mut outputs: Vec<AddressData> = Vec::new();
        let mut funds_per_output: u64 = 0;

        // keep choosing random subset from source until each output will recieve at least 1 coin
        // since zero output is not allowed
        // TODO: randomize funds per output
        while funds_per_output == 0 {
            outputs = Self::choose_random_subset(source, gen)
                .iter()
                .cloned()
                .map(|x| x.address_data)
                .collect();
            funds_per_output = total_input_funds / outputs.len() as u64;
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
