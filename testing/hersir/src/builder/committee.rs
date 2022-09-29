use crate::{builder::Wallet, config::CommitteeTemplate};
use chain_crypto::bech32::Bech32;
use chain_vote::{
    committee::MemberCommunicationPublicKey, MemberCommunicationKey, MemberPublicKey,
};
use jormungandr_lib::interfaces::CommitteeIdDef;
use thor::{
    wallet::committee::{
        CommitteeCommunicationData, CommitteeCommunicationDataManager, CommitteeMembershipData,
        CommitteeMembershipDataManager,
    },
    CommitteeDataManager, WalletAlias,
};

pub fn generate_committee_data(
    wallets: &[Wallet],
    committees_templates: &[CommitteeTemplate],
) -> Result<CommitteeDataManager, Error> {
    let mut comm_manager = CommitteeCommunicationDataManager::default();
    let mut member_manager = CommitteeMembershipDataManager::default();

    let mut rng = rand::thread_rng();

    for committee_template in committees_templates.iter() {
        match committee_template {
            CommitteeTemplate::Generated {
                alias,
                member_pk,
                communication_pk,
            } => {
                let id: CommitteeIdDef = wallets
                    .iter()
                    .find(|w| w.has_alias(alias))
                    .ok_or_else(|| Error::CannotFindAlias(alias.clone()))?
                    .committee_id()?
                    .into();

                if let Some(member_pk) = member_pk {
                    member_manager
                        .committees_mut()
                        .push(CommitteeMembershipData::public(
                            id,
                            MemberPublicKey::try_from_bech32_str(member_pk)?,
                        ));
                } else if let Some(communication_pk) = communication_pk {
                    comm_manager
                        .committees_mut()
                        .push(CommitteeCommunicationData::public(
                            id,
                            MemberCommunicationPublicKey::try_from_bech32_str(communication_pk)?,
                        ));
                } else {
                    comm_manager
                        .committees_mut()
                        .push(CommitteeCommunicationData::private(
                            id,
                            MemberCommunicationKey::new(&mut rng),
                        ));
                }
            }
            CommitteeTemplate::External {
                id: hex,
                member_pk,
                communication_pk,
            } => {
                let id = CommitteeIdDef::from_hex(hex).unwrap();

                if let Some(member_pk) = member_pk {
                    member_manager
                        .committees_mut()
                        .push(CommitteeMembershipData::public(
                            id,
                            MemberPublicKey::try_from_bech32_str(member_pk)?,
                        ));
                } else if let Some(communication_pk) = communication_pk {
                    comm_manager
                        .committees_mut()
                        .push(CommitteeCommunicationData::public(
                            id,
                            MemberCommunicationPublicKey::try_from_bech32_str(communication_pk)?,
                        ));
                } else {
                    comm_manager
                        .committees_mut()
                        .push(CommitteeCommunicationData::private(
                            id,
                            MemberCommunicationKey::new(&mut rng),
                        ));
                }
            }
        }
    }

    Ok(CommitteeDataManager {
        communication: comm_manager.into(),
        membership: member_manager,
    })
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("cannot find alias '{0}' for any defined wallet")]
    CannotFindAlias(WalletAlias),
    #[error(transparent)]
    Wallet(#[from] crate::builder::settings::wallet::Error),
    #[error(transparent)]
    Bech3(#[from] chain_crypto::bech32::Error),
}
