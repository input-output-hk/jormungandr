use super::WalletError as Error;
use crate::testing::network_builder::WalletAlias;
use assert_fs::fixture::{ChildPath, PathChild};
use bech32::FromBase32;
use bech32::ToBase32;
use chain_impl_mockchain::{
    certificate::{DecryptedPrivateTally, DecryptedPrivateTallyProposal},
    vote::VotePlanStatus,
};
use chain_vote::{
    committee::{
        ElectionPublicKey, MemberCommunicationKey, MemberCommunicationPublicKey, MemberPublicKey,
        MemberState,
    },
    tally::{Crs, OpeningVoteKey},
};
use jormungandr_lib::crypto::account::Identifier;
use rand_core::{CryptoRng, RngCore};
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::Write;

pub const COMMUNICATION_SK_HRP: &str = "p256k1_vcommsk";
pub const MEMBER_SK_HRP: &str = "p256k1_membersk";
pub const ENCRYPTING_VOTE_PK_HRP: &str = "p256k1_votepk";

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

        writeln!(
            file,
            "{}",
            bech32::encode(
                COMMUNICATION_SK_HRP,
                self.communication_key.to_bytes().to_base32()
            )
            .unwrap()
        )
        .unwrap()
    }

    fn write_member_secret_key(&self, directory: &ChildPath) {
        let path = directory.child("member_secret_key.sk");
        let mut file = File::create(path.path()).unwrap();
        writeln!(
            file,
            "{}",
            bech32::encode(
                MEMBER_SK_HRP,
                self.member_secret_key().to_bytes().to_base32()
            )
            .unwrap()
        )
        .unwrap()
    }
}

pub trait ElectionPublicKeyExtension {
    fn to_base32(&self) -> Result<String, bech32::Error>;
}

impl ElectionPublicKeyExtension for ElectionPublicKey {
    fn to_base32(&self) -> Result<String, bech32::Error> {
        bech32::encode(ENCRYPTING_VOTE_PK_HRP, self.to_bytes().to_base32())
    }
}

pub fn election_key_from_base32(key: &str) -> Result<ElectionPublicKey, Error> {
    let (hrp, data) = bech32::decode(&key).map_err(Error::InvalidBech32)?;
    if hrp != ENCRYPTING_VOTE_PK_HRP {
        return Err(Error::InvalidBech32Key {
            expected: ENCRYPTING_VOTE_PK_HRP.to_string(),
            actual: hrp,
        });
    }
    let key_bin = Vec::<u8>::from_base32(&data)?;
    chain_vote::ElectionPublicKey::from_bytes(&key_bin).ok_or(Error::ElectionPublicKey)
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
        writeln!(file, "{}", self.election_public_key().to_base32().unwrap()).unwrap()
    }

    pub fn member_public_keys(&self) -> Vec<MemberPublicKey> {
        self.data.values().map(|x| x.member_public_key()).collect()
    }

    pub fn decrypt_tally(&self, vote_plan_status: &VotePlanStatus) -> DecryptedPrivateTally {
        let encrypted_tally = vote_plan_status
            .proposals
            .iter()
            .map(|proposal| {
                let tally_state = proposal.tally.as_ref().unwrap();
                let encrypted_tally = tally_state.private_encrypted().unwrap().0.clone();
                let max_votes = tally_state.private_total_power().unwrap();
                (encrypted_tally, max_votes)
            })
            .collect::<Vec<_>>();

        let absolute_max_votes = encrypted_tally
            .iter()
            .map(|(_encrypted_tally, max_votes)| *max_votes)
            .max()
            .unwrap();
        let table = chain_vote::TallyOptimizationTable::generate(absolute_max_votes);

        let proposals = encrypted_tally
            .into_iter()
            .map(|(encrypted_tally, max_votes)| {
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
                    .unwrap()
                    .decrypt_tally(max_votes, &table)
                    .unwrap();
                DecryptedPrivateTallyProposal {
                    decrypt_shares: decrypt_shares.into_boxed_slice(),
                    tally_result: tally.votes.into_boxed_slice(),
                }
            })
            .collect::<Vec<_>>();

        DecryptedPrivateTally::new(proposals)
    }
}
