use assert_fs::fixture::{ChildPath, PathChild};
use chain_crypto::bech32::Bech32;
use chain_vote::{
    committee::MemberCommunicationPublicKey, tally::OpeningVoteKey, MemberCommunicationKey,
    MemberPublicKey,
};
use jormungandr_lib::interfaces::CommitteeIdDef;
use std::{fmt, fs::File, io::Write};
#[derive(Clone)]
pub struct CommitteeCommunicationData {
    pub id: CommitteeIdDef,
    pub communication_public_key: MemberCommunicationPublicKey,
    communication_secret_key: Option<MemberCommunicationKey>,
}

impl CommitteeCommunicationData {
    pub fn write_to(&self, directory: &ChildPath) {
        std::fs::create_dir_all(directory.path()).unwrap();
        write_to(
            self.communication_public_key.to_bech32_str(),
            directory,
            "communication.pk",
        );
        if let Some(private) = &self.communication_secret_key {
            write_to(private.to_bech32_str(), directory, "communication.sk");
        }
    }

    pub fn public(
        id: CommitteeIdDef,
        communication_public_key: MemberCommunicationPublicKey,
    ) -> Self {
        Self {
            id,
            communication_public_key,
            communication_secret_key: None,
        }
    }

    pub fn private(id: CommitteeIdDef, private: MemberCommunicationKey) -> Self {
        let communication_public_key = private.to_public();
        Self {
            id,
            communication_public_key,
            communication_secret_key: Some(private),
        }
    }
}

#[derive(Clone)]
pub struct CommitteeMembershipData {
    pub id: CommitteeIdDef,
    pub member_public_key: MemberPublicKey,
    member_secret_key: Option<OpeningVoteKey>,
}

impl CommitteeMembershipData {
    pub(crate) fn member_secret_key(&self) -> Result<OpeningVoteKey, Error> {
        self.member_secret_key
            .clone()
            .ok_or(Error::NoMemberSecretKeyDefined)
    }
}

impl fmt::Debug for CommitteeMembershipData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("CommitteeMembershipData")?;
        if let Some(member_secret_key) = &self.member_secret_key {
            write!(
                f,
                "member secret key: {}",
                member_secret_key.to_bech32_str()
            )?;
        }
        write!(
            f,
            "member public key: {}",
            self.member_public_key.to_bech32_str()
        )?;
        f.write_str(")")
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("no secret key defined")]
    NoMemberSecretKeyDefined,
}

impl CommitteeMembershipData {
    pub fn write_to(&self, directory: &ChildPath) {
        std::fs::create_dir_all(directory.path()).unwrap();
        write_to(
            self.member_public_key.to_bech32_str(),
            directory,
            "member.pk",
        );
        if let Some(private) = &self.member_secret_key {
            write_to(private.to_bech32_str(), directory, "member.sk");
        }
    }

    pub fn public(id: CommitteeIdDef, member_public_key: MemberPublicKey) -> Self {
        Self {
            id,
            member_public_key,
            member_secret_key: None,
        }
    }

    pub fn private(id: CommitteeIdDef, private: OpeningVoteKey) -> Self {
        let member_public_key = private.to_public();
        Self {
            id,
            member_public_key,
            member_secret_key: Some(private),
        }
    }
}

impl fmt::Debug for CommitteeCommunicationData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("CommitteeCommunicationData")?;
        if let Some(communication_secret_key) = &self.communication_secret_key {
            write!(
                f,
                "communication secret key: {}",
                communication_secret_key.to_bech32_str()
            )?;
        }
        write!(
            f,
            "communication public key: {}",
            self.communication_public_key.to_bech32_str()
        )?;
        f.write_str(")")
    }
}

fn write_to(key: String, directory: &ChildPath, name: &str) {
    let path = directory.child(name);
    let mut file = File::create(path.path()).unwrap();
    writeln!(file, "{}", key).unwrap()
}
