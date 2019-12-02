use crate::{
    account::Ledger as AccountLedger,
    block::{ConsensusVersion, HeaderId},
    config::ConfigParam,
    fee::LinearFee,
    fragment::{config::ConfigParams, Fragment, FragmentId},
    header::BlockDate,
    leadership::bft::LeaderId,
    ledger::{Error, Ledger, LedgerParameters},
    milli::Milli,
    testing::data::{AddressData, AddressDataValue,Wallet},
    transaction::{Output, TxBuilder},
    utxo::{Entry, Iter},
    value::Value,
    stake::PoolsState
};
use chain_addr::{Address, Discrimination};
use chain_crypto::*;
use std::collections::HashMap;

#[derive(Clone)]
pub struct ConfigBuilder {
    slot_duration: u8,
    slots_per_epoch: u32,
    active_slots_coeff: Milli,
    discrimination: Discrimination,
    linear_fee: Option<LinearFee>,
    leaders: Vec<LeaderId>,
    seed: u64,
}

impl ConfigBuilder {
    pub fn new(seed: u64) -> Self {
        ConfigBuilder {
            slot_duration: 20,
            slots_per_epoch: 21600,
            active_slots_coeff: Milli::HALF,
            discrimination: Discrimination::Test,
            leaders: Vec::new(),
            linear_fee: None,
            seed,
        }
    }

    pub fn with_discrimination(mut self, discrimination: Discrimination) -> Self {
        self.discrimination = discrimination;
        self
    }

    pub fn with_slot_duration(mut self, slot_duration: u8) -> Self {
        self.slot_duration = slot_duration;
        self
    }

    pub fn with_leaders(mut self, leaders: &Vec<LeaderId>) -> Self {
        self.leaders.extend(leaders.iter().cloned());
        self
    }

    pub fn with_fee(mut self, linear_fee: LinearFee) -> Self {
        self.linear_fee = Some(linear_fee);
        self
    }

    pub fn with_slots_per_epoch(mut self, slots_per_epoch: u32) -> Self {
        self.slots_per_epoch = slots_per_epoch;
        self
    }

    pub fn with_active_slots_coeff(mut self, active_slots_coeff: Milli) -> Self {
        self.active_slots_coeff = active_slots_coeff;
        self
    }

    fn create_single_bft_leader() -> LeaderId {
        let leader_prv_key: SecretKey<Ed25519Extended> =
            SecretKey::generate(rand_os::OsRng::new().unwrap());
        let leader_pub_key = leader_prv_key.to_public();
        leader_pub_key.into()
    }

    pub fn normalize(&mut self) {
        // TODO remove rng: make this creation deterministic
        if self.leaders.is_empty() {
            self.leaders.push(Self::create_single_bft_leader());
        }
    }

    pub fn build(self) -> ConfigParams {
        let mut ie = ConfigParams::new();
        ie.push(ConfigParam::Discrimination(self.discrimination));
        ie.push(ConfigParam::ConsensusVersion(ConsensusVersion::Bft));

        for leader_id in self.leaders.iter().cloned() {
            ie.push(ConfigParam::AddBftLeader(leader_id));
        }

        if self.linear_fee.is_some() {
            ie.push(ConfigParam::LinearFee(self.linear_fee.clone().unwrap()));
        }

        ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));
        ie.push(ConfigParam::SlotDuration(self.slot_duration));
        ie.push(ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(
            self.active_slots_coeff,
        ));
        ie.push(ConfigParam::SlotsPerEpoch(self.slots_per_epoch));
        ie.push(ConfigParam::KESUpdateSpeed(3600 * 12));
        ie
    }
}

pub struct LedgerBuilder {
    cfg_builder: ConfigBuilder,
    cfg_params: ConfigParams,
    fragments: Vec<Fragment>,
    certs: Vec<Fragment>,
    faucets: Vec<AddressDataValue>,
    utxo_declaration: Vec<UtxoDeclaration>,
}

pub type UtxoDeclaration = Output<Address>;

#[derive(Clone)]
pub struct UtxoDb {
    db: HashMap<(FragmentId, u8), UtxoDeclaration>,
}

impl UtxoDb {
    pub fn find_fragments(&self, decl: &UtxoDeclaration) -> Vec<(FragmentId, u8)> {
        self.db
            .iter()
            .filter_map(|(k, v)| if v == decl { Some(k.clone()) } else { None })
            .collect()
    }

    pub fn get(&self, key: &(FragmentId, u8)) -> Option<&UtxoDeclaration> {
        self.db.get(key)
    }
}

impl LedgerBuilder {
    pub fn from_config(mut cfg_builder: ConfigBuilder) -> Self {
        cfg_builder.normalize();
        let cfg_params = cfg_builder.clone().build();
        Self {
            cfg_builder,
            cfg_params,
            faucets: Vec::new(),
            utxo_declaration: Vec::new(),
            fragments: Vec::new(),
            certs: Vec::new(),
        }
    }

    pub fn fragment(mut self, f: Fragment) -> Self {
        self.fragments.push(f);
        self
    }

    pub fn fragments(mut self, f: &[Fragment]) -> Self {
        self.fragments.extend_from_slice(f);
        self
    }

    pub fn certs(mut self, f: &[Fragment]) -> Self {
        self.certs.extend_from_slice(f);
        self
    }

    // add a fragment that pre-fill the address with a specific value at ledger start
    pub fn prefill_address(self, address: Address, value: Value) -> Self {
        self.prefill_output(Output { address, value })
    }

    pub fn prefill_output(self, output: Output<Address>) -> Self {
        let tx = TxBuilder::new()
            .set_nopayload()
            .set_ios(&[], &[output])
            .set_witnesses(&[])
            .set_payload_auth(&());
        self.fragment(Fragment::Transaction(tx))
    }

    pub fn prefill_outputs(self, outputs: &[Output<Address>]) -> Self {
        let tx = TxBuilder::new()
            .set_nopayload()
            .set_ios(&[], outputs)
            .set_witnesses(&[])
            .set_payload_auth(&());
        self.fragment(Fragment::Transaction(tx))
    }

    pub fn faucet_value(mut self, value: Value) -> Self {
        self.faucets.push(AddressDataValue::account(
            self.cfg_builder.discrimination,
            value,
        ));
        self
    }

    pub fn initial_fund(mut self, fund: &AddressDataValue) -> Self {
        if fund.is_utxo() {
            self = self.utxos(&[fund.make_output()]);
        } else {
            self = self.faucet(&fund);
        }
        self
    }

    pub fn initial_funds(mut self, funds: &Vec<AddressDataValue>) -> Self {
        for fund in funds {
            self = self.initial_fund(fund);
        }
        self
    }

    pub fn faucet(mut self, faucet: &AddressDataValue) -> Self {
        self.faucets.push(faucet.clone());
        self
    }

    pub fn faucets_wallets(mut self, faucets: Vec<&Wallet>) -> Self {
        self.faucets.extend(faucets.iter().cloned().map(|x| x.as_account()));
        self
    }

    pub fn faucets(mut self, faucets: &Vec<AddressDataValue>) -> Self {
        self.faucets.extend(faucets.iter().cloned());
        self
    }

    pub fn utxos(mut self, decls: &[UtxoDeclaration]) -> Self {
        self.utxo_declaration.extend_from_slice(decls);
        self
    }

    pub fn build(mut self) -> Result<TestLedger, Error> {
        let block0_hash = HeaderId::hash_bytes(&[1, 2, 3]);
        let outputs: Vec<Output<Address>> = self.faucets.iter().map(|x| x.make_output()).collect();
        self = self.prefill_outputs(&outputs);

        let utxodb = if self.utxo_declaration.len() > 0 {
            let mut db = HashMap::new();

            // TODO subdivide utxo_declaration in group of 254 elements
            // and repeatdly create fragment
            assert!(self.utxo_declaration.len() < 254);
            let group = self.utxo_declaration;
            {
                let tx = TxBuilder::new()
                    .set_nopayload()
                    .set_ios(&[], &group)
                    .set_witnesses(&[])
                    .set_payload_auth(&());
                let fragment = Fragment::Transaction(tx);
                let fragment_id = fragment.hash();

                for (idx, o) in group.iter().enumerate() {
                    let m = db.insert((fragment_id.clone(), idx as u8), o.clone());
                    assert!(m.is_none());
                }

                self.fragments.push(fragment);
            }
            UtxoDb { db }
        } else {
            UtxoDb { db: HashMap::new() }
        };

        let cfg = self.cfg_params.clone();

        let mut fragments = Vec::new();
        fragments.push(Fragment::Initial(self.cfg_params));
        fragments.extend_from_slice(&self.fragments);
        fragments.extend_from_slice(&self.certs);

        let faucets = self.faucets.clone();
        Ledger::new(block0_hash, &fragments).map(|ledger| {
            let parameters = ledger.get_ledger_parameters();
            TestLedger {
                cfg,
                faucets,
                ledger,
                block0_hash,
                utxodb,
                parameters,
            }
        })
    }
}
#[derive(Clone)]
pub struct TestLedger {
    pub block0_hash: HeaderId,
    pub cfg: ConfigParams,
    pub faucets: Vec<AddressDataValue>,
    pub ledger: Ledger,
    pub parameters: LedgerParameters,
    pub utxodb: UtxoDb,
}

impl TestLedger {
    pub fn apply_transaction(&mut self, fragment: Fragment) -> Result<(), Error> {
        let fragment_id = fragment.hash();
        match fragment {
            Fragment::Transaction(tx) => {
                match self.ledger.clone().apply_transaction(
                    &fragment_id,
                    &tx.as_slice(),
                    &self.parameters,
                ) {
                    Err(err) => Err(err),
                    Ok((ledger, _)) => {
                        // TODO more bookkeeping for accounts and utxos
                        self.ledger = ledger;
                        Ok(())
                    }
                }
            }
            _ => panic!("test ledger apply transaction only supports transaction type for now"),
        }
    }

    pub fn apply_fragment(&mut self, fragment: &Fragment, date: BlockDate) -> Result<(), Error> {
        self.ledger = self
            .ledger
            .clone()
            .apply_fragment(&self.parameters, fragment, date)?;
        Ok(())
    }

    pub fn total_funds(&self) -> Value {
        let utxo_total = Value(self.ledger.utxos().map(|x| x.output.value.0).sum::<u64>());
        let accounts_total = self.ledger.accounts().get_total_value().unwrap();
        (utxo_total + accounts_total).unwrap()
    }

    pub fn find_utxo_for_address<'a>(
        &'a self,
        address_data: &AddressData,
    ) -> Option<Entry<'a, Address>> {
        let entry = self
            .utxos()
            .find(|x| x.output.address == address_data.address);
        entry
    }

    pub fn accounts(&self) -> &AccountLedger {
        &self.ledger.accounts()
    }

    pub fn utxos<'a>(&'a self) -> Iter<'a, Address> {
        self.ledger.utxos()
    }

    pub fn fee(&self) -> LinearFee {
        self.parameters.fees
    }

    pub fn delegation(&self) -> PoolsState{
        self.ledger.delegation().clone()
        
    }
}

impl Into<Ledger> for TestLedger {
    fn into(self) -> Ledger {
        self.ledger
    }
}
