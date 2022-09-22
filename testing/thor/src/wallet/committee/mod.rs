use assert_fs::{fixture::ChildPath, prelude::PathChild};
use chain_impl_mockchain::{
    certificate::{
        DecryptedPrivateTally, DecryptedPrivateTallyError, DecryptedPrivateTallyProposal,
    },
    vote::VotePlanStatus,
};
use chain_vote::{
    committee::MemberCommunicationPublicKey, tally::batch_decrypt, Crs, ElectionPublicKey,
    MemberCommunicationKey, MemberPublicKey, MemberState,
};
use jormungandr_lib::{crypto::account::Identifier, interfaces::CommitteeIdDef};
use rand_core::{CryptoRng, OsRng, RngCore};

mod single;

pub use crate::wallet::committee::single::{CommitteeCommunicationData, CommitteeMembershipData};

#[derive(Clone, Debug, Default)]
pub struct CommitteeCommunicationDataManager {
    committees: Vec<CommitteeCommunicationData>,
}

impl CommitteeCommunicationDataManager {
    pub fn committees_mut(&mut self) -> &mut Vec<CommitteeCommunicationData> {
        &mut self.committees
    }

    pub fn write_to(&self, directory: &ChildPath) {
        let path = directory.child("communication");
        self.committees.iter().for_each(|x| x.write_to(&path));
    }
    pub fn committees(&self) -> &Vec<CommitteeCommunicationData> {
        &self.committees
    }

    pub fn membership_data<RNG>(
        &self,
        crs: Crs,
        threshold: usize,
        mut rng: &mut RNG,
    ) -> CommitteeMembershipDataManager
    where
        RNG: RngCore + CryptoRng,
    {
        let keys: Vec<MemberCommunicationPublicKey> = self
            .committees
            .iter()
            .map(|x| x.communication_public_key.clone())
            .collect();

        let committees = self
            .committees
            .iter()
            .cloned()
            .enumerate()
            .map(|(idx, data)| {
                let ms = MemberState::new(&mut rng, threshold, &crs, &keys, idx);
                CommitteeMembershipData::private(data.id, ms.secret_key().clone())
            })
            .collect();

        CommitteeMembershipDataManager { committees }
    }
    pub fn new(committees: Vec<CommitteeCommunicationData>) -> Self {
        Self { committees }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CommitteeMembershipDataManager {
    pub(crate) committees: Vec<CommitteeMembershipData>,
}

impl CommitteeMembershipDataManager {
    pub fn committees_mut(&mut self) -> &mut Vec<CommitteeMembershipData> {
        &mut self.committees
    }
    pub fn write_to(&self, directory: &ChildPath) {
        let path = directory.child("member");
        self.committees.iter().for_each(|x| x.write_to(&path));
    }

    pub fn committees(&self) -> &Vec<CommitteeMembershipData> {
        &self.committees
    }
    pub fn new(committees: Vec<CommitteeMembershipData>) -> Self {
        Self { committees }
    }
}

#[derive(Clone, Debug)]
pub struct CommitteeDataManager {
    pub communication: Option<CommitteeCommunicationDataManager>,
    pub membership: CommitteeMembershipDataManager,
}

impl CommitteeDataManager {
    pub fn new(
        comm_data: Vec<CommitteeCommunicationData>,
        member_data: Vec<CommitteeMembershipData>,
    ) -> Self {
        Self {
            communication: {
                if comm_data.is_empty() {
                    None
                } else {
                    Some(CommitteeCommunicationDataManager::new(comm_data))
                }
            },
            membership: CommitteeMembershipDataManager::new(member_data),
        }
    }
    pub fn private(mut rng: &mut OsRng, wallets: Vec<Identifier>, threshold: usize) -> Self {
        let comm_data = CommitteeCommunicationDataManager::new(
            wallets
                .iter()
                .map(|id| {
                    CommitteeCommunicationData::private(
                        CommitteeIdDef::from_hex(&id.to_hex()).unwrap(),
                        MemberCommunicationKey::new(&mut rng),
                    )
                })
                .collect(),
        );

        let membership = comm_data.membership_data(
            Crs::from_hash("Dummy shared string".as_bytes()),
            threshold,
            &mut rng,
        );

        Self {
            communication: Some(comm_data),
            membership,
        }
    }

    pub fn member_public_keys(&self) -> Vec<MemberPublicKey> {
        self.membership
            .committees()
            .iter()
            .cloned()
            .map(|x| x.member_public_key)
            .collect()
    }

    pub fn committee_ids(&self) -> Vec<CommitteeIdDef> {
        let mut ids: Vec<CommitteeIdDef> =
            self.membership.committees().iter().map(|x| x.id).collect();
        if let Some(comm) = &self.communication {
            let additional_ids: Vec<CommitteeIdDef> =
                comm.committees.iter().map(|x| x.id).collect();
            ids.extend(additional_ids);
        };
        ids
    }

    pub fn write_to(&self, directory: &ChildPath) {
        if let Some(communication) = &self.communication {
            communication.write_to(directory);
        }
        self.membership.write_to(directory);
    }

    pub fn election_key(&self) -> ElectionPublicKey {
        let pks: Vec<MemberPublicKey> = self
            .membership
            .committees()
            .iter()
            .map(|x| x.member_public_key.clone())
            .collect();
        ElectionPublicKey::from_participants(&pks)
    }

    pub fn decrypt_tally(
        &self,
        vote_plan_status: &VotePlanStatus,
    ) -> Result<DecryptedPrivateTally, DecryptedPrivateTallyError> {
        let (shares, tallies): (Vec<_>, Vec<_>) = vote_plan_status
            .proposals
            .iter()
            .map(|proposal| {
                let tally_state = &proposal.tally;
                let encrypted_tally = tally_state.private_encrypted().unwrap().clone();
                let decrypt_shares = self
                    .membership
                    .committees
                    .iter()
                    .map(|member| member.member_secret_key().unwrap())
                    .map(|secret_key| {
                        encrypted_tally.partial_decrypt(&mut rand::thread_rng(), &secret_key)
                    })
                    .collect::<Vec<_>>();
                let tally = encrypted_tally
                    .validate_partial_decryptions(
                        &vote_plan_status.committee_public_keys,
                        &decrypt_shares,
                    )
                    .unwrap();
                (decrypt_shares, tally)
            })
            .unzip();
        let tallies = batch_decrypt(&tallies).unwrap();

        DecryptedPrivateTally::new(
            tallies
                .into_iter()
                .zip(shares)
                .map(
                    |(tally_result, decrypt_shares)| DecryptedPrivateTallyProposal {
                        tally_result: tally_result.votes.into_boxed_slice(),
                        decrypt_shares: decrypt_shares.into_boxed_slice(),
                    },
                )
                .collect(),
        )
        .map_err(Into::into)
    }
}
