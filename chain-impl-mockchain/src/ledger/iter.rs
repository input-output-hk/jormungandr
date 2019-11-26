use super::ledger::{Error, Ledger, LedgerStaticParameters};
use super::pots::{self, Pots};
use crate::block::LeadersParticipationRecord;
use crate::config::ConfigParam;
use crate::header::{BlockDate, ChainLength};
use crate::stake::PoolsState;
use crate::{account, legacy, multisig, setting, update, utxo};
use chain_addr::Address;
use chain_time::TimeEra;
use std::sync::Arc;

pub enum Entry<'a> {
    Globals(Globals),
    Pot(pots::Entry),
    Utxo(utxo::Entry<'a, Address>),
    OldUtxo(utxo::Entry<'a, legacy::OldAddress>),
    Account(
        (
            &'a account::Identifier,
            &'a crate::accounting::account::AccountState<()>,
        ),
    ),
    ConfigParam(ConfigParam),
    UpdateProposal(
        (
            &'a crate::update::UpdateProposalId,
            &'a crate::update::UpdateProposalState,
        ),
    ),
    MultisigAccount(
        (
            &'a crate::multisig::Identifier,
            &'a crate::accounting::account::AccountState<()>,
        ),
    ),
    MultisigDeclaration(
        (
            &'a crate::multisig::Identifier,
            &'a crate::multisig::Declaration,
        ),
    ),
    StakePool(
        (
            &'a crate::certificate::PoolId,
            &'a crate::certificate::PoolRegistration,
        ),
    ),
    LeaderParticipation((&'a crate::certificate::PoolId, &'a u32)),
}

pub struct Globals {
    pub date: BlockDate,
    pub chain_length: ChainLength,
    pub static_params: LedgerStaticParameters,
    pub era: TimeEra,
}

enum IterState<'a> {
    Initial,
    Utxo(utxo::Iter<'a, Address>),
    OldUtxo(utxo::Iter<'a, legacy::OldAddress>),
    Accounts(crate::accounting::account::Iter<'a, account::Identifier, ()>),
    ConfigParams(Vec<ConfigParam>),
    UpdateProposals(
        std::collections::btree_map::Iter<
            'a,
            crate::update::UpdateProposalId,
            crate::update::UpdateProposalState,
        >,
    ),
    MultisigAccounts(crate::accounting::account::Iter<'a, crate::multisig::Identifier, ()>),
    MultisigDeclarations(
        imhamt::HamtIter<'a, crate::multisig::Identifier, crate::multisig::Declaration>,
    ),
    StakePools(
        imhamt::HamtIter<'a, crate::certificate::PoolId, crate::certificate::PoolRegistration>,
    ),
    Pots(pots::Entries<'a>),
    LeaderParticipations(imhamt::HamtIter<'a, crate::certificate::PoolId, u32>),
    Done,
}

pub struct LedgerIterator<'a> {
    ledger: &'a Ledger,
    state: IterState<'a>,
}

impl<'a> Iterator for LedgerIterator<'a> {
    type Item = Entry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.state {
            IterState::Initial => {
                self.state = IterState::Utxo(self.ledger.utxos.iter());
                Some(Entry::Globals(Globals {
                    date: self.ledger.date,
                    chain_length: self.ledger.chain_length,
                    static_params: (*self.ledger.static_params).clone(),
                    era: self.ledger.era.clone(),
                }))
            }
            IterState::Utxo(iter) => match iter.next() {
                None => {
                    self.state = IterState::OldUtxo(self.ledger.oldutxos.iter());
                    self.next()
                }
                Some(x) => Some(Entry::Utxo(x)),
            },
            IterState::OldUtxo(iter) => match iter.next() {
                None => {
                    self.state = IterState::Accounts(self.ledger.accounts.iter());
                    self.next()
                }
                Some(x) => Some(Entry::OldUtxo(x)),
            },
            IterState::Accounts(iter) => match iter.next() {
                None => {
                    self.state = IterState::ConfigParams(self.ledger.settings.to_config_params().0);
                    self.next()
                }
                Some(x) => Some(Entry::Account(x)),
            },
            IterState::ConfigParams(params) => {
                if let Some(param) = params.pop() {
                    Some(Entry::ConfigParam(param))
                } else {
                    self.state = IterState::UpdateProposals(self.ledger.updates.proposals.iter());
                    self.next()
                }
            }
            IterState::UpdateProposals(iter) => match iter.next() {
                None => {
                    self.state = IterState::MultisigAccounts(self.ledger.multisig.iter_accounts());
                    self.next()
                }
                Some(x) => Some(Entry::UpdateProposal(x)),
            },
            IterState::MultisigAccounts(iter) => match iter.next() {
                None => {
                    self.state =
                        IterState::MultisigDeclarations(self.ledger.multisig.iter_declarations());
                    self.next()
                }
                Some(x) => Some(Entry::MultisigAccount(x)),
            },
            IterState::MultisigDeclarations(iter) => match iter.next() {
                None => {
                    self.state = IterState::StakePools(self.ledger.delegation.stake_pools.iter());
                    self.next()
                }
                Some(x) => Some(Entry::MultisigDeclaration(x)),
            },
            IterState::StakePools(iter) => match iter.next() {
                None => {
                    self.state = IterState::Pots(self.ledger.pots.entries());
                    self.next()
                }
                Some(x) => Some(Entry::StakePool(x)),
            },
            IterState::Pots(iter) => match iter.next() {
                None => {
                    self.state = IterState::LeaderParticipations(self.ledger.leaders_log.iter());
                    self.next()
                }
                Some(x) => Some(Entry::Pot(x)),
            },
            IterState::LeaderParticipations(iter) => match iter.next() {
                None => {
                    self.state = IterState::Done;
                    self.next()
                }
                Some(x) => Some(Entry::LeaderParticipation(x)),
            },
            IterState::Done => None,
        }
    }
}

impl Ledger {
    pub fn iter<'a>(&'a self) -> LedgerIterator<'a> {
        LedgerIterator {
            ledger: self,
            state: IterState::Initial,
        }
    }
}

impl<'a> std::iter::FromIterator<Entry<'a>> for Result<Ledger, Error> {
    fn from_iter<I: IntoIterator<Item = Entry<'a>>>(iter: I) -> Self {
        let mut utxos = std::collections::HashMap::new();
        let mut oldutxos = std::collections::HashMap::new();
        let mut accounts = vec![];
        let mut config_params = crate::fragment::ConfigParams::new();
        let mut updates = update::UpdateState::new();
        let mut multisig_accounts = vec![];
        let mut multisig_declarations = vec![];
        let delegation = PoolsState::new();
        let mut globals = None;
        let mut pots = Pots::zero();
        let mut leaders_log = LeadersParticipationRecord::new();

        for entry in iter {
            match entry {
                Entry::Globals(globals2) => {
                    globals = Some(globals2);
                    // FIXME: check duplicate
                }
                Entry::Utxo(entry) => {
                    utxos
                        .entry(entry.fragment_id)
                        .or_insert(vec![])
                        .push((entry.output_index, entry.output.clone()));
                }
                Entry::OldUtxo(entry) => {
                    oldutxos
                        .entry(entry.fragment_id)
                        .or_insert(vec![])
                        .push((entry.output_index, entry.output.clone()));
                }
                Entry::Account((account_id, account_state)) => {
                    accounts.push((account_id.clone(), account_state.clone()));
                }
                Entry::ConfigParam(param) => {
                    config_params.push(param.clone());
                }
                Entry::UpdateProposal((proposal_id, proposal_state)) => {
                    updates
                        .proposals
                        .insert(proposal_id.clone(), proposal_state.clone());
                }
                Entry::MultisigAccount((account_id, account_state)) => {
                    multisig_accounts.push((account_id.clone(), account_state.clone()));
                }
                Entry::MultisigDeclaration((id, decl)) => {
                    multisig_declarations.push((id.clone(), decl.clone()));
                }
                Entry::StakePool((pool_id, pool_state)) => {
                    delegation
                        .stake_pools
                        .insert(pool_id.clone(), pool_state.clone())
                        .unwrap();
                }
                Entry::Pot(ent) => pots.set_from_entry(&ent),
                Entry::LeaderParticipation((pool_id, pool_participation)) => leaders_log
                    .set_for(pool_id.clone(), *pool_participation)
                    .unwrap(),
            }
        }

        let globals = globals.ok_or(Error::IncompleteLedger)?;

        Ok(Ledger {
            utxos: utxos.into_iter().collect(),
            oldutxos: oldutxos.into_iter().collect(),
            accounts: accounts.into_iter().collect(),
            settings: setting::Settings::new().apply(&config_params)?,
            updates,
            multisig: multisig::Ledger::restore(multisig_accounts, multisig_declarations),
            delegation,
            static_params: Arc::new(globals.static_params),
            date: globals.date,
            chain_length: globals.chain_length,
            era: globals.era,
            pots: pots,
            leaders_log,
        })
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        ledger::{Entry, Ledger},
        testing::{ConfigBuilder, LedgerBuilder},
        value::Value,
    };

    fn print_from_iter(ledger: &Ledger) {
        for item in ledger.iter() {
            match item {
                Entry::Globals(globals) => {
                    println!(
                    "Globals date={} length={} block0_hash={} start_time={:?} discr={} kes_update_speed={}",
                    globals.date,
                    globals.chain_length,
                    globals.static_params.block0_initial_hash,
                    globals.static_params.block0_start_time,
                    globals.static_params.discrimination,
                    globals.static_params.kes_update_speed,
                );
                }
                Entry::Utxo(entry) => {
                    println!(
                        "Utxo {} {} {}",
                        entry.fragment_id, entry.output_index, entry.output
                    );
                }
                Entry::OldUtxo(entry) => {
                    println!(
                        "OldUtxo {} {} {}",
                        entry.fragment_id, entry.output_index, entry.output
                    );
                }
                Entry::Account((id, state)) => {
                    println!(
                        "Account {} {} {:?} {}",
                        id,
                        u32::from(state.counter),
                        state.delegation,
                        state.value,
                    );
                }
                Entry::ConfigParam(param) => {
                    println!(
                        "ConfigParam {:?} {:?}",
                        crate::config::Tag::from(&param),
                        param,
                    );
                }
                Entry::UpdateProposal((id, state)) => {
                    println!(
                        "UpdateProposal {} {:?} {} {:?}",
                        id, state.proposal, state.proposal_date, state.votes
                    );
                }
                Entry::MultisigAccount((id, state)) => {
                    println!(
                        "MultisigAccount {} {} {:?} {}",
                        id,
                        u32::from(state.counter),
                        state.delegation,
                        state.value,
                    );
                }
                Entry::MultisigDeclaration((id, decl)) => {
                    println!(
                        "MultisigDeclaration {} {} {}",
                        id,
                        decl.threshold(),
                        decl.total(),
                    );
                }
                Entry::StakePool((id, info)) => {
                    println!(
                        "StakePool {} {} {:?} {:?}",
                        id, info.serial, info.owners, info.keys,
                    );
                }
                Entry::Pot(entry) => {
                    println!("Pot {:?}", entry);
                }
                Entry::LeaderParticipation((pool_id, pool_record)) => {
                    println!("LeaderParticipation {} {}", pool_id, pool_record);
                }
            }
        }
    }

    #[test]
    pub fn iterate() {
        let testledger = LedgerBuilder::from_config(ConfigBuilder::new(0))
            .faucet_value(Value(42000))
            .build()
            .expect("cannot build test ledger");

        let ledger = testledger.ledger;

        let ledger2: Result<Ledger, _> = ledger.iter().collect();
        let ledger2 = ledger2.unwrap();

        print_from_iter(&ledger);

        assert!(ledger == ledger2)
    }
}
