use crate::{
    account::{AccountAlg, SpendingCounter},
    block::{ConsensusVersion, HeaderId},
    config::ConfigParam,
    fee::LinearFee,
    fragment::{config::ConfigParams, Fragment},
    key::EitherEd25519SecretKey,
    leadership::bft::LeaderId,
    ledger::{Error, Ledger},
    milli::Milli,
    transaction::{Input, Output, TxBuilder, TransactionAuthData, Witness},
    value::Value,
};
use chain_addr::{Address, Discrimination};
use chain_crypto::*;
use std::vec::Vec;

//use crate::testing::{data::AddressDataValue};

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
    faucet_value: Option<Value>,
    seed: u64,
}

pub struct Faucet {
    pub block0_hash: HeaderId,
    st: SpendingCounter,
    discrimination: Discrimination,
    secret_key: SecretKey<AccountAlg>,
    pub initial_value: Value,
}

impl Faucet {
    pub fn get_address(&self) -> Address {
        let pk = self.secret_key.to_public();
        Address(self.discrimination, chain_addr::Kind::Account(pk))
    }

    pub fn get_input_of(&self, value: Value) -> Input {
        Input::from_account_public_key(self.secret_key.to_public(), value)
    }

    pub fn make_witness<'a>(&mut self, tad: TransactionAuthData<'a>) -> Witness {
        let sc = self.st;
        self.st = self.st.increment().expect("faucet use more than expected");

        let sk = EitherEd25519SecretKey::Normal(self.secret_key.clone());
        Witness::new_account(&self.block0_hash, &tad.hash(), &sc, &sk)
    }
}

impl LedgerBuilder {
    pub fn from_config(mut cfg_builder: ConfigBuilder) -> Self {
        cfg_builder.normalize();
        let cfg_params = cfg_builder.clone().build();
        Self {
            seed: cfg_builder.seed,
            cfg_builder,
            cfg_params,
            faucet_value: None,
            fragments: Vec::new(),
        }
    }

    fn randbuf(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.seed as u8;
            self.seed = self.seed.wrapping_mul(2862933555777941757u64).wrapping_add(3037000493);
        }
    }

    fn account_secret_key(&mut self) -> SecretKey<AccountAlg> {
        let mut sk = [0u8;32];
        self.randbuf(&mut sk);
        SecretKey::from_binary(&sk).unwrap()
    }

    pub fn fragment(mut self, f: Fragment) -> Self {
        self.fragments.push(f);
        self
    }

    pub fn fragments(mut self, f: &[Fragment]) -> Self {
        self.fragments.extend_from_slice(f);
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

    pub fn faucet(mut self, faucet_value: Value) -> Self {
        self.faucet_value = Some(faucet_value);
        self
    }

    pub fn build(mut self) -> Result<TestLedger, Error> {
        let block0_hash = HeaderId::hash_bytes(&[1, 2, 3]);

        // push the faucet
        let faucet = match self.faucet_value {
            None => None,
            Some(val) => {
                let secret_key = self.account_secret_key();
                let faucet = Faucet { block0_hash, st: SpendingCounter::zero(), discrimination: self.cfg_builder.discrimination, secret_key, initial_value: val };
                self = self.prefill_address(faucet.get_address(), val);
                Some(faucet) 
            }
        };

        let cfg = self.cfg_params.clone();

        let mut fragments = Vec::new();
        fragments.push(Fragment::Initial(self.cfg_params));
        fragments.extend_from_slice(&self.fragments);

        Ledger::new(block0_hash, &fragments).map(|ledger| TestLedger { cfg, faucet, ledger, block0_hash })
    }
}

pub struct TestLedger {
    pub block0_hash: HeaderId, 
    pub cfg: ConfigParams,
    pub faucet: Option<Faucet>,
    pub ledger: Ledger,
}
