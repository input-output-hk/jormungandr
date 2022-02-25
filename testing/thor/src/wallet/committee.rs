use crate::wallet::WalletAlias;
use assert_fs::fixture::{ChildPath, PathChild};
use chain_crypto::bech32::Bech32;
use chain_impl_mockchain::{
    certificate::{
        DecryptedPrivateTally, DecryptedPrivateTallyError, DecryptedPrivateTallyProposal,
    },
    vote::VotePlanStatus,
};
use chain_vote::{
    committee::{
        ElectionPublicKey, MemberCommunicationKey, MemberCommunicationPublicKey, MemberPublicKey,
        MemberState,
    },
    tally::{batch_decrypt, Crs, OpeningVoteKey},
};
use jormungandr_lib::crypto::account::Identifier;
use rand_core::{CryptoRng, RngCore};
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::Write;

#[derive(Clone)]
pub struct PrivateVoteCommitteeData {
    alias: String,
    communication_key: MemberCommunicationKey,
    member_secret_key: OpeningVoteKey,
    member_public_key: MemberPublicKey,
    election_public_key: ElectionPublicKey,
}

impl PrivateVoteCommitteeData {
    pub fn new(
        alias: String,
        communication_key: MemberCommunicationKey,
        member_secret_key: OpeningVoteKey,
        member_public_key: MemberPublicKey,
        election_public_key: ElectionPublicKey,
    ) -> Self {
        Self {
            alias,
            communication_key,
            member_secret_key,
            member_public_key,
            election_public_key,
        }
    }

    pub fn member_public_key(&self) -> MemberPublicKey {
        self.member_public_key.clone()
    }

    pub fn member_secret_key(&self) -> OpeningVoteKey {
        self.member_secret_key.clone()
    }

    pub fn election_public_key(&self) -> ElectionPublicKey {
        self.election_public_key.clone()
    }

    pub fn alias(&self) -> String {
        self.alias.clone()
    }

    pub fn write_to(&self, directory: ChildPath) {
        std::fs::create_dir_all(directory.path()).unwrap();
        self.write_communication_key(&directory);
        self.write_member_secret_key(&directory);
    }

    fn write_communication_key(&self, directory: &ChildPath) {
        let path = directory.child("communication_key.sk");
        let mut file = File::create(path.path()).unwrap();

        writeln!(file, "{}", self.communication_key.to_bech32_str()).unwrap()
    }

    fn write_member_secret_key(&self, directory: &ChildPath) {
        let path = directory.child("member_secret_key.sk");
        let mut file = File::create(path.path()).unwrap();
        writeln!(file, "{}", self.member_secret_key().to_bech32_str()).unwrap()
    }
}

impl fmt::Debug for PrivateVoteCommitteeData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("PrivateVoteCommitteeData")?;
        write!(
            f,
            "communication key: {:?}",
            self.communication_key.to_public().to_bytes()
        )?;
        write!(
            f,
            "member public key: {:?}",
            self.member_public_key.to_bytes()
        )?;
        f.write_str(")")
    }
}

#[derive(Clone, Debug)]
pub struct PrivateVoteCommitteeDataManager {
    data: HashMap<Identifier, PrivateVoteCommitteeData>,
}

impl PrivateVoteCommitteeDataManager {
    pub fn new<RNG>(
        mut rng: &mut RNG,
        committees: Vec<(WalletAlias, Identifier)>,
        threshold: usize,
    ) -> Self
    where
        RNG: RngCore + CryptoRng,
    {
        let mut buf = [0; 32];
        rand::thread_rng().fill_bytes(&mut buf);
        let crs = Crs::from_hash(&buf);
        let mut data = HashMap::new();

        let communication_secret_keys: Vec<MemberCommunicationKey> =
            std::iter::from_fn(|| Some(MemberCommunicationKey::new(&mut rng)))
                .take(committees.len())
                .collect();
        let communication_public_keys: Vec<MemberCommunicationPublicKey> =
            communication_secret_keys
                .iter()
                .map(|x| x.to_public())
                .collect();

        for (index, (alias, pk)) in committees.iter().enumerate() {
            let ms = MemberState::new(&mut rng, threshold, &crs, &communication_public_keys, index);

            let communication_secret_key = communication_secret_keys.get(index).unwrap();
            let election_public_key =
                ElectionPublicKey::from_participants(&[ms.public_key().clone()]);

            data.insert(
                pk.clone(),
                PrivateVoteCommitteeData::new(
                    alias.clone(),
                    communication_secret_key.clone(),
                    ms.secret_key().clone(),
                    ms.public_key().clone(),
                    election_public_key,
                ),
            );
        }

        Self { data }
    }

    pub fn get(&self, identifier: &Identifier) -> Option<&PrivateVoteCommitteeData> {
        self.data.get(identifier)
    }

    pub fn election_public_key(&self) -> ElectionPublicKey {
        chain_vote::ElectionPublicKey::from_participants(&self.member_public_keys())
    }

    pub fn members(&self) -> Vec<PrivateVoteCommitteeData> {
        self.data.values().cloned().collect()
    }

    pub fn write_to(&self, directory: ChildPath) -> std::io::Result<()> {
        std::fs::create_dir_all(directory.path()).unwrap();
        self.write_election_public_key(&directory);
        for (id, data) in self.data.iter() {
            let item_directory = directory.child(id.to_bech32_str());
            data.write_to(item_directory);
        }
        Ok(())
    }

    fn write_election_public_key(&self, directory: &ChildPath) {
        let path = directory.child("election_public_key.sk");
        let mut file = File::create(path.path()).unwrap();
        writeln!(file, "{}", self.election_public_key().to_bech32_str()).unwrap()
    }

    pub fn member_public_keys(&self) -> Vec<MemberPublicKey> {
        self.data.values().map(|x| x.member_public_key()).collect()
    }

    pub fn decrypt_tally(
        &self,
        vote_plan_status: &VotePlanStatus,
    ) -> Result<DecryptedPrivateTally, DecryptedPrivateTallyError> {
        let (shares, tallies): (Vec<_>, Vec<_>) = vote_plan_status
            .proposals
            .iter()
            .map(|proposal| {
                let tally_state = proposal.tally.as_ref().unwrap();
                let encrypted_tally = tally_state.private_encrypted().unwrap().0.clone();
                let decrypt_shares = self
                    .members()
                    .iter()
                    .map(|member| member.member_secret_key())
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
