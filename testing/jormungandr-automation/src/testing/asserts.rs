use chain_addr::AddressReadable;
use chain_impl_mockchain::key::Hash;
use jormungandr_lib::interfaces::{Initial, InitialUTxO, PrivateTallyState, Tally, VotePlanStatus};
use std::str::FromStr;

pub trait VotePlanStatusAssert {
    fn assert_proposal_tally(&self, vote_plan_id: String, index: u8, expected: Vec<u64>);
}

impl VotePlanStatusAssert for Vec<VotePlanStatus> {
    fn assert_proposal_tally(&self, vote_plan_id: String, index: u8, expected: Vec<u64>) {
        let vote_plan_status = self
            .iter()
            .find(|c_vote_plan| c_vote_plan.id == Hash::from_str(&vote_plan_id).unwrap().into())
            .unwrap();

        let tally = &vote_plan_status
            .proposals
            .iter()
            .find(|x| x.index == index)
            .unwrap()
            .tally;

        match tally {
            Tally::Public { result } => assert_eq!(expected, result.results()),
            Tally::Private { state } => match state {
                PrivateTallyState::Encrypted { .. } => {
                    panic!("expected decrypted private tally state")
                }
                PrivateTallyState::Decrypted { result, .. } => {
                    assert_eq!(expected, result.results())
                }
            },
        }
    }
}

pub trait InitialsAssert {
    fn assert_contains(&self, entry: InitialUTxO);
    fn assert_not_contain(&self, entry: InitialUTxO);
}

impl InitialsAssert for Vec<Initial> {
    fn assert_contains(&self, expected: InitialUTxO) {
        let address_readable =
            AddressReadable::from_address("ca", &expected.address.clone().into()).to_string();
        for initial in self.iter() {
            if let Initial::Fund(initial_utxos) = initial {
                if let Some(entry) = initial_utxos.iter().find(|x| x.address == expected.address) {
                    assert_eq!(
                        entry.value, expected.value,
                        "Address {} found but value is different",
                        address_readable
                    );
                    return;
                }
            }
        }
        panic!("Address {} not found", address_readable);
    }

    fn assert_not_contain(&self, entry: InitialUTxO) {
        let address_readable =
            AddressReadable::from_address("ca", &entry.address.clone().into()).to_string();
        for initial in self.iter() {
            if let Initial::Fund(initial_utxos) = initial {
                if initial_utxos.iter().any(|x| x.address == entry.address) {
                    panic!("Address {} found, while it shouldn't", address_readable);
                }
            }
        }
    }
}
