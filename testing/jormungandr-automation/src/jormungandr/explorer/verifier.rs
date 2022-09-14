use super::data::{
    address::AddressAddress, settings::SettingsSettingsFees, transaction_by_id_certificates::*,
    transactions_by_address::TransactionsByAddressTipTransactionsByAddress,
    vote_plan_by_id::VotePlanByIdVotePlan,
};
use crate::jormungandr::explorer::{
    data::{transaction_by_id_certificates::PayloadType as expPayloadType, vote_plan_by_id},
    vote_plan_by_id::VotePlanByIdVotePlanProposalsTally::*,
};
use bech32::FromBase32;
use chain_addr::AddressReadable;
use chain_crypto::{Ed25519, PublicKey};
use chain_impl_mockchain::{
    account::DelegationType,
    certificate::*,
    chaintypes::ConsensusType,
    config::{ConfigParam::*, RewardParams},
    fee::LinearFee,
    fragment::Fragment,
    testing::data::Wallet,
    transaction::{AccountIdentifier, InputEnum, Transaction},
    vote,
    vote::{Choice, PayloadType},
};
use jormungandr_lib::interfaces::{
    Address, FragmentStatus, PrivateTallyState, Tally, VotePlanStatus,
};
use std::{collections::HashMap, num::NonZeroU64};
use thiserror::Error;
use vote_plan_by_id::VotePlanByIdVotePlanProposalsVotesEdgesNodePayload::*;

#[derive(Debug, Error)]
pub enum VerifierError {
    #[error("Not implemented")]
    Unimplemented,
    #[error("Invalid certificate, received: {received}")]
    InvalidCertificate { received: String },
}
pub struct ExplorerVerifier;

impl ExplorerVerifier {
    pub fn assert_transaction_certificates(
        fragment: Fragment,
        explorer_transaction: TransactionByIdCertificatesTransaction,
    ) -> Result<(), VerifierError> {
        if explorer_transaction.certificate.is_none() {
            if let Fragment::Transaction(fragment_transaction) = fragment {
                Self::assert_transaction_params(fragment_transaction, explorer_transaction)
                    .unwrap();
                Ok(())
            } else {
                Err(VerifierError::InvalidCertificate {
                    received: "Transaction".to_string(),
                })
            }
        } else {
            let explorer_certificate = explorer_transaction.certificate.as_ref().unwrap();
            match explorer_certificate {
                TransactionByIdCertificatesTransactionCertificate::StakeDelegation(
                    explorer_cert,
                ) => {
                    if let Fragment::StakeDelegation(fragment_cert) = fragment {
                        Self::assert_transaction_params(
                            fragment_cert.clone(),
                            explorer_transaction.clone(),
                        )
                        .unwrap();
                        Self::assert_stake_delegation(fragment_cert, explorer_cert.clone())
                            .unwrap();
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "StakeDelegation".to_string(),
                        })
                    }
                }
                TransactionByIdCertificatesTransactionCertificate::OwnerStakeDelegation(
                    explorer_cert,
                ) => {
                    if let Fragment::OwnerStakeDelegation(fragment_cert) = fragment {
                        Self::assert_transaction_params(
                            fragment_cert.clone(),
                            explorer_transaction.clone(),
                        )
                        .unwrap();
                        Self::assert_owner_delegation(fragment_cert, explorer_cert.clone())
                            .unwrap();
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "OwnerStakeDelegation".to_string(),
                        })
                    }
                }
                TransactionByIdCertificatesTransactionCertificate::PoolRegistration(
                    explorer_cert,
                ) => {
                    if let Fragment::PoolRegistration(fragment_cert) = fragment {
                        Self::assert_transaction_params(
                            fragment_cert.clone(),
                            explorer_transaction.clone(),
                        )
                        .unwrap();
                        Self::assert_pool_registration(fragment_cert, explorer_cert.clone());
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "PoolRegistration".to_string(),
                        })
                    }
                }
                TransactionByIdCertificatesTransactionCertificate::PoolRetirement(
                    explorer_cert,
                ) => {
                    if let Fragment::PoolRetirement(fragment_cert) = fragment {
                        Self::assert_transaction_params(
                            fragment_cert.clone(),
                            explorer_transaction.clone(),
                        )
                        .unwrap();
                        Self::assert_pool_retirement(fragment_cert, explorer_cert.clone());
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "PoolRetirement".to_string(),
                        })
                    }
                }
                TransactionByIdCertificatesTransactionCertificate::PoolUpdate(explorer_cert) => {
                    if let Fragment::PoolUpdate(fragment_cert) = fragment {
                        Self::assert_transaction_params(
                            fragment_cert.clone(),
                            explorer_transaction.clone(),
                        )
                        .unwrap();
                        Self::assert_pool_update(fragment_cert, explorer_cert.clone());
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "PoolUpdate".to_string(),
                        })
                    }
                }
                TransactionByIdCertificatesTransactionCertificate::VotePlan(explorer_cert) => {
                    if let Fragment::VotePlan(fragment_cert) = fragment {
                        Self::assert_transaction_params(
                            fragment_cert.clone(),
                            explorer_transaction.clone(),
                        )
                        .unwrap();
                        Self::assert_vote_plan(fragment_cert, explorer_cert.clone());
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "VotePlan".to_string(),
                        })
                    }
                }
                TransactionByIdCertificatesTransactionCertificate::VoteCast(explorer_cert) => {
                    if let Fragment::VoteCast(fragment_cert) = fragment {
                        Self::assert_transaction_params(
                            fragment_cert.clone(),
                            explorer_transaction.clone(),
                        )
                        .unwrap();
                        Self::assert_vote_cast(fragment_cert, explorer_cert.clone());
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "VoteCast".to_string(),
                        })
                    }
                }
                TransactionByIdCertificatesTransactionCertificate::VoteTally(explorer_cert) => {
                    if let Fragment::VoteTally(fragment_cert) = fragment {
                        Self::assert_transaction_params(
                            fragment_cert.clone(),
                            explorer_transaction.clone(),
                        )
                        .unwrap();
                        Self::assert_vote_tally(fragment_cert, explorer_cert.clone());
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "VoteTally".to_string(),
                        })
                    }
                }
                TransactionByIdCertificatesTransactionCertificate::UpdateProposal(
                    explorer_cert,
                ) => {
                    if let Fragment::UpdateProposal(fragment_cert) = fragment {
                        Self::assert_transaction_params(
                            fragment_cert.clone(),
                            explorer_transaction.clone(),
                        )
                        .unwrap();
                        Self::assert_update_proposal(fragment_cert, explorer_cert.clone());
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "UpdateProposal".to_string(),
                        })
                    }
                }
                TransactionByIdCertificatesTransactionCertificate::UpdateVote(explorer_cert) => {
                    if let Fragment::UpdateVote(fragment_cert) = fragment {
                        Self::assert_transaction_params(
                            fragment_cert.clone(),
                            explorer_transaction.clone(),
                        )
                        .unwrap();
                        Self::assert_update_vote(fragment_cert, explorer_cert.clone());
                        Ok(())
                    } else {
                        Err(VerifierError::InvalidCertificate {
                            received: "UpdateVote".to_string(),
                        })
                    }
                }
                TransactionByIdCertificatesTransactionCertificate::MintToken(_) => {
                    Err(VerifierError::InvalidCertificate {
                        received: "MintToken can be only in block0".to_string(),
                    })
                }
                TransactionByIdCertificatesTransactionCertificate::EvmMapping(_) => {
                    //Not implemented because of the bug EAS-238
                    Err(VerifierError::Unimplemented)
                }
            }
        }
    }

    fn assert_transaction_params<P>(
        fragment_transaction: Transaction<P>,
        explorer_transaction: TransactionByIdCertificatesTransaction,
    ) -> Result<(), VerifierError> {
        assert_eq!(
            fragment_transaction.as_slice().nb_inputs(),
            explorer_transaction.inputs.len() as u8
        );

        if fragment_transaction.as_slice().nb_inputs() > 0 {
            let mut fragment_accounts = vec![];

            for fragment_input in fragment_transaction.as_slice().inputs().iter() {
                match fragment_input.to_enum() {
                    InputEnum::AccountInput(account_id, amount) => {
                        fragment_accounts.push((
                            account_id.to_single_account().unwrap().to_string(),
                            amount.to_string(),
                        ));
                        Ok(())
                    }
                    InputEnum::UtxoInput(_) => Err(VerifierError::Unimplemented),
                }
                .unwrap()
            }

            let mut explorer_accounts = vec![];

            for explorer_inputs in explorer_transaction.inputs.iter() {
                let adr =
                    AddressReadable::from_string_anyprefix(&explorer_inputs.address.id).unwrap();
                explorer_accounts.push((
                    adr.to_address().public_key().unwrap().to_string(),
                    explorer_inputs.amount.clone(),
                ));
            }

            let matching_inputs = fragment_accounts
                .iter()
                .zip(explorer_accounts.iter())
                .filter(|&(a, b)| a == b)
                .count();
            assert_eq!(matching_inputs, explorer_transaction.inputs.len());
        };

        assert_eq!(
            fragment_transaction.as_slice().nb_outputs(),
            explorer_transaction.outputs.len() as u8
        );

        if fragment_transaction.as_slice().nb_outputs() > 0 {
            let mut fragment_accounts = vec![];

            for fragment_output in fragment_transaction.as_slice().outputs().iter() {
                fragment_accounts.push((
                    fragment_output.address.public_key().unwrap().to_string(),
                    fragment_output.value.to_string(),
                ));
            }

            let mut explorer_accounts = vec![];

            for explorer_outputs in explorer_transaction.outputs.iter() {
                let adr =
                    AddressReadable::from_string_anyprefix(&explorer_outputs.address.id).unwrap();
                explorer_accounts.push((
                    adr.to_address().public_key().unwrap().to_string(),
                    explorer_outputs.amount.clone(),
                ));
            }

            let matching_outputs = fragment_accounts
                .iter()
                .zip(explorer_accounts.iter())
                .filter(|&(a, b)| a == b)
                .count();
            assert_eq!(matching_outputs, explorer_transaction.outputs.len());
        };
        Ok(())
    }

    fn assert_pool_registration(
        fragment_cert: Transaction<PoolRegistration>,
        explorer_cert: TransactionByIdCertificatesTransactionCertificateOnPoolRegistration,
    ) {
        let pool_cert = fragment_cert.as_slice().payload().into_payload();

        assert_eq!(pool_cert.to_id().to_string(), explorer_cert.pool.id);
        assert_eq!(
            u64::from(pool_cert.start_validity),
            explorer_cert.start_validity.parse::<u64>().unwrap()
        );
        if pool_cert.reward_account.is_some() {
            if let AccountIdentifier::Single(id) = pool_cert.reward_account.as_ref().unwrap() {
                assert_eq!(id.to_string(), explorer_cert.reward_account.unwrap().id);
            }
        }

        assert_eq!(
            pool_cert.rewards.ratio.numerator,
            explorer_cert
                .rewards
                .ratio
                .numerator
                .parse::<u64>()
                .unwrap()
        );
        assert_eq!(
            pool_cert.rewards.ratio.denominator,
            explorer_cert
                .rewards
                .ratio
                .denominator
                .parse::<NonZeroU64>()
                .unwrap()
        );
        if pool_cert.rewards.max_limit.is_some() {
            assert_eq!(
                pool_cert.rewards.max_limit.unwrap(),
                explorer_cert
                    .rewards
                    .max_limit
                    .unwrap()
                    .parse::<NonZeroU64>()
                    .unwrap()
            );
        }

        assert_eq!(
            pool_cert.management_threshold(),
            explorer_cert.management_threshold as u8
        );

        assert_eq!(pool_cert.owners.len(), explorer_cert.owners.len());

        let owners_matching = pool_cert
            .owners
            .iter()
            .zip(explorer_cert.owners.iter())
            .filter(|&(a, b)| *a == Self::decode_bech32_pk(b))
            .count();

        assert_eq!(pool_cert.owners.len(), owners_matching);

        assert_eq!(pool_cert.operators.len(), explorer_cert.operators.len());

        let operators_matching = pool_cert
            .operators
            .iter()
            .zip(explorer_cert.operators.iter())
            .filter(|&(a, b)| *a == Self::decode_bech32_pk(b))
            .count();

        assert_eq!(pool_cert.operators.len(), operators_matching);

        assert!(explorer_cert.pool.retirement.is_none());
    }

    fn assert_stake_delegation(
        fragment_cert: Transaction<StakeDelegation>,
        explorer_cert: TransactionByIdCertificatesTransactionCertificateOnStakeDelegation,
    ) -> Result<(), VerifierError> {
        let deleg_cert = fragment_cert.as_slice().payload().into_payload();
        let adr = AddressReadable::from_string_anyprefix(&explorer_cert.account.id).unwrap();
        assert_eq!(
            deleg_cert
                .account_id
                .to_single_account()
                .unwrap()
                .to_string(),
            adr.to_address().public_key().unwrap().to_string()
        );

        match deleg_cert.delegation {
            DelegationType::NonDelegated => Err(VerifierError::Unimplemented),
            DelegationType::Full(pool_id) => {
                assert_eq!(explorer_cert.pools.len(), 1);
                assert_eq!(pool_id.to_string(), explorer_cert.pools[0].id);
                Ok(())
            }
            DelegationType::Ratio(deleg) => {
                assert_eq!(explorer_cert.pools.len(), deleg.pools().len());
                let pools_matching = explorer_cert
                    .pools
                    .iter()
                    .zip(deleg.pools().iter())
                    .filter(|&(a, b)| a.id == b.0.to_string())
                    .count();
                assert_eq!(pools_matching, explorer_cert.pools.len());
                Ok(())
            }
        }
    }
    fn assert_owner_delegation(
        fragment_cert: Transaction<OwnerStakeDelegation>,
        explorer_cert: TransactionByIdCertificatesTransactionCertificateOnOwnerStakeDelegation,
    ) -> Result<(), VerifierError> {
        let owner_cert = fragment_cert.as_slice().payload().into_payload();

        match owner_cert.delegation {
            DelegationType::NonDelegated => Err(VerifierError::Unimplemented),
            DelegationType::Full(pool_id) => {
                assert_eq!(explorer_cert.pools.len(), 1);
                assert_eq!(pool_id.to_string(), explorer_cert.pools[0].id);
                Ok(())
            }
            DelegationType::Ratio(deleg) => {
                assert_eq!(explorer_cert.pools.len(), deleg.pools().len());
                let pools_matching = explorer_cert
                    .pools
                    .iter()
                    .zip(deleg.pools().iter())
                    .filter(|&(a, b)| a.id == b.0.to_string())
                    .count();
                assert_eq!(pools_matching, explorer_cert.pools.len());
                Ok(())
            }
        }
    }

    fn assert_pool_retirement(
        fragment_cert: Transaction<PoolRetirement>,
        explorer_cert: TransactionByIdCertificatesTransactionCertificateOnPoolRetirement,
    ) {
        let ret_cert = fragment_cert.as_slice().payload().into_payload();
        assert_eq!(ret_cert.pool_id.to_string(), explorer_cert.pool_id);
        assert_eq!(
            u64::from(ret_cert.retirement_time),
            explorer_cert.retirement_time.parse::<u64>().unwrap()
        );
    }

    fn assert_pool_update(
        fragment_cert: Transaction<PoolUpdate>,
        explorer_cert: TransactionByIdCertificatesTransactionCertificateOnPoolUpdate,
    ) {
        let update_cert = fragment_cert.as_slice().payload().into_payload();
        assert_eq!(update_cert.pool_id.to_string(), explorer_cert.pool_id);
        assert_eq!(
            u64::from(update_cert.new_pool_reg.start_validity),
            explorer_cert.start_validity.parse::<u64>().unwrap()
        );
    }

    fn assert_vote_plan(
        fragment_cert: Transaction<VotePlan>,
        explorer_cert: TransactionByIdCertificatesTransactionCertificateOnVotePlan,
    ) {
        let vote_plan_cert = fragment_cert.as_slice().payload().into_payload();
        assert_eq!(
            explorer_cert.vote_start.epoch.id.parse::<u32>().unwrap(),
            vote_plan_cert.vote_start().epoch
        );
        assert_eq!(
            explorer_cert.vote_start.slot.parse::<u32>().unwrap(),
            vote_plan_cert.vote_start().slot_id
        );
        assert_eq!(
            explorer_cert.vote_end.epoch.id.parse::<u32>().unwrap(),
            vote_plan_cert.vote_end().epoch
        );
        assert_eq!(
            explorer_cert.vote_end.slot.parse::<u32>().unwrap(),
            vote_plan_cert.vote_end().slot_id
        );
        assert_eq!(
            explorer_cert.committee_end.epoch.id.parse::<u32>().unwrap(),
            vote_plan_cert.committee_end().epoch
        );
        assert_eq!(
            explorer_cert.committee_end.slot.parse::<u32>().unwrap(),
            vote_plan_cert.committee_end().slot_id
        );

        match vote_plan_cert.payload_type() {
            PayloadType::Public => {
                assert!(matches!(explorer_cert.payload_type, expPayloadType::PUBLIC))
            }
            PayloadType::Private => assert!(matches!(
                explorer_cert.payload_type,
                expPayloadType::PRIVATE
            )),
        }

        assert_eq!(
            explorer_cert.proposals.len(),
            vote_plan_cert.proposals().len()
        );
        let matching_proposal = explorer_cert
            .proposals
            .iter()
            .zip(vote_plan_cert.proposals().iter())
            .filter(|&(a, b)| a.external_id == b.external_id().to_string())
            .count();
        assert_eq!(explorer_cert.proposals.len(), matching_proposal);
    }

    fn assert_vote_cast(
        fragment_cert: Transaction<VoteCast>,
        explorer_cert: TransactionByIdCertificatesTransactionCertificateOnVoteCast,
    ) {
        let vote_cast_cert = fragment_cert.as_slice().payload().into_payload();

        assert_eq!(
            explorer_cert.vote_plan,
            vote_cast_cert.vote_plan().to_string()
        );
        assert_eq!(
            explorer_cert.proposal_index as u8,
            vote_cast_cert.proposal_index()
        );
    }

    fn assert_vote_tally(
        fragment_cert: Transaction<VoteTally>,
        explorer_cert: TransactionByIdCertificatesTransactionCertificateOnVoteTally,
    ) {
        let vote_tally_cert = fragment_cert.as_slice().payload().into_payload();
        assert_eq!(explorer_cert.vote_plan, vote_tally_cert.id().to_string());
    }

    fn assert_update_proposal(
        fragment_cert: Transaction<UpdateProposal>,
        explorer_cert: TransactionByIdCertificatesTransactionCertificateOnUpdateProposal,
    ) {
        let update_proposal_cert = fragment_cert.as_slice().payload().into_payload();
        assert_eq!(
            Self::decode_bech32_pk(&explorer_cert.proposer_id.id),
            *update_proposal_cert.proposer_id().as_public_key()
        );
        assert_eq!(
            explorer_cert.changes.config_params.len(),
            update_proposal_cert.changes().iter().len()
        );

        //for each parameter in the update_proposal_certificate check that there is only one parameter
        //of the corrisponding type in the explorer query answer and that the parameters have the same value
        for update_proposal_param in update_proposal_cert.changes().iter() {
            match update_proposal_param {
                Block0Date(certificate_param) => {
                    let matching_params = explorer_cert
                        .changes
                        .config_params
                        .iter()
                        .filter(|&config_param| {
                            matches!(config_param, configParam::Block0Date(explorer_param)
                        if explorer_param.block0_date as u64 == certificate_param.0)
                        })
                        .count();
                    assert_eq!(matching_params, 1);
                }
                Discrimination(certificate_param) => {
                    let matching_params = explorer_cert.changes.config_params.iter()
                        .filter(|&config_param| matches!(config_param, configParam::Discrimination(explorer_param)
                        if { match explorer_param.discrimination{
                            DiscriminationEnum::PRODUCTION => {matches!(certificate_param, chain_addr::Discrimination::Production)},
                            DiscriminationEnum::TEST => {matches!(certificate_param, chain_addr::Discrimination::Test)},
                            DiscriminationEnum::Other(_) => false,
                        }})).count();
                    assert_eq!(matching_params, 1);
                }
                ConsensusVersion(certificate_param) => {
                    let matching_params = explorer_cert.changes.config_params.iter()
                        .filter(|&config_param| matches!(config_param, configParam::ConsensusType(explorer_param)
                        if { match explorer_param.consensus_type{
                            ConsensusTypeEnum::BFT => {matches!(certificate_param, ConsensusType::Bft)},
                            ConsensusTypeEnum::GENESIS_PRAOS => {matches!(certificate_param, ConsensusType::GenesisPraos)},
                            ConsensusTypeEnum::Other(_) => false,
                        }})).count();
                    assert_eq!(matching_params, 1);
                }
                SlotsPerEpoch(certificate_param) => {
                    let matching_params = explorer_cert
                        .changes
                        .config_params
                        .iter()
                        .filter(|&config_param| {
                            matches!(config_param, configParam::SlotsPerEpoch(explorer_param)
                        if explorer_param.slots_per_epoch as u32 == *certificate_param)
                        })
                        .count();
                    assert_eq!(matching_params, 1);
                }
                SlotDuration(certificate_param) => {
                    let matching_params = explorer_cert
                        .changes
                        .config_params
                        .iter()
                        .filter(|&config_param| {
                            matches!(config_param, configParam::SlotDuration(explorer_param)
                        if explorer_param.slot_duration as u8 == *certificate_param)
                        })
                        .count();
                    assert_eq!(matching_params, 1);
                }
                EpochStabilityDepth(certificate_param) => {
                    let matching_params = explorer_cert
                        .changes
                        .config_params
                        .iter()
                        .filter(|&config_param| {
                            matches!(config_param, configParam::EpochStabilityDepth(explorer_param)
                        if explorer_param.epoch_stability_depth as u32 == *certificate_param)
                        })
                        .count();
                    assert_eq!(matching_params, 1);
                }
                ConsensusGenesisPraosActiveSlotsCoeff(certificate_param) => {
                    let matching_params = explorer_cert
                        .changes
                        .config_params
                        .iter()
                        .filter(|&config_param| {
                            matches!(config_param, configParam::Milli(explorer_param)
                        if explorer_param.milli as u64 == certificate_param.to_millis())
                        })
                        .count();
                    assert_eq!(matching_params, 1);
                }
                BlockContentMaxSize(certificate_param) => {
                    let matching_params = explorer_cert
                        .changes
                        .config_params
                        .iter()
                        .filter(|&config_param| {
                            matches!(config_param, configParam::BlockContentMaxSize(explorer_param)
                        if explorer_param.block_content_max_size as u32 == *certificate_param)
                        })
                        .count();
                    assert_eq!(matching_params, 1);
                }
                AddBftLeader(certificate_param) => {
                    let matching_params = explorer_cert.changes.config_params.iter()
                        .filter(|&config_param| matches!(config_param, configParam::AddBftLeader(explorer_param)
                        if explorer_param.add_bft_leader.id == certificate_param.as_public_key().to_string())).count();
                    assert_eq!(matching_params, 1);
                }
                RemoveBftLeader(certificate_param) => {
                    let matching_params = explorer_cert.changes.config_params.iter()
                        .filter(|&config_param| matches!(config_param, configParam::RemoveBftLeader(explorer_param)
                        if explorer_param.remove_bft_leader.id == certificate_param.as_public_key().to_string())).count();
                    assert_eq!(matching_params, 1);
                }
                LinearFee(certificate_param) => {
                    let matching_params = explorer_cert.changes.config_params.iter()
                        .filter(|&config_param| matches!(config_param, configParam::LinearFee(explorer_param)
                        if {explorer_param.certificate  as u64 == certificate_param.certificate &&
                            explorer_param.coefficient as u64 == certificate_param.coefficient &&
                            explorer_param.constant as u64 == certificate_param.constant &&
                            explorer_param.per_certificate_fees.certificate_owner_stake_delegation.unwrap() as u64 == u64::from(certificate_param.per_certificate_fees.certificate_owner_stake_delegation.unwrap()) &&
                            explorer_param.per_certificate_fees.certificate_pool_registration.unwrap() as u64 == u64::from(certificate_param.per_certificate_fees.certificate_pool_registration.unwrap()) &&
                            explorer_param.per_certificate_fees.certificate_stake_delegation.unwrap() as u64 == u64::from(certificate_param.per_certificate_fees.certificate_stake_delegation.unwrap()) &&
                            explorer_param.per_vote_certificate_fees.certificate_vote_cast.unwrap() as u64 == u64::from(certificate_param.per_vote_certificate_fees.certificate_vote_cast.unwrap()) &&
                            explorer_param.per_vote_certificate_fees.certificate_vote_plan.unwrap() as u64 == u64::from(certificate_param.per_vote_certificate_fees.certificate_vote_plan.unwrap())
                        })).count();
                    assert_eq!(matching_params, 1);
                }
                ProposalExpiration(certificate_param) => {
                    let matching_params = explorer_cert
                        .changes
                        .config_params
                        .iter()
                        .filter(|&config_param| {
                            matches!(config_param, configParam::ProposalExpiration(explorer_param)
                        if explorer_param.proposal_expiration as u32 == *certificate_param)
                        })
                        .count();
                    assert_eq!(matching_params, 1);
                }
                KesUpdateSpeed(certificate_param) => {
                    let matching_params = explorer_cert
                        .changes
                        .config_params
                        .iter()
                        .filter(|&config_param| {
                            matches!(config_param, configParam::KesUpdateSpeed(explorer_param)
                        if explorer_param.kes_update_speed as u32 == *certificate_param)
                        })
                        .count();
                    assert_eq!(matching_params, 1);
                }
                TreasuryAdd(certificate_param) => {
                    let matching_params = explorer_cert
                        .changes
                        .config_params
                        .iter()
                        .filter(|&config_param| {
                            matches!(config_param, configParam::TreasuryAdd(explorer_param)
                        if explorer_param.treasury_add == certificate_param.to_string())
                        })
                        .count();
                    assert_eq!(matching_params, 1);
                }
                TreasuryParams(certificate_param) => {
                    let matching_params = explorer_cert.changes.config_params.iter()
                        .filter(|&config_param| matches!(config_param, configParam::TreasuryParams(explorer_param)
                        if {explorer_param.treasury_params.fixed == certificate_param.fixed.to_string() &&
                            explorer_param.treasury_params.ratio.numerator.parse::<u64>().unwrap() == certificate_param.ratio.numerator &&
                            explorer_param.treasury_params.ratio.denominator.parse::<u64>().unwrap() == u64::from(certificate_param.ratio.denominator) &&
                            explorer_param.treasury_params.max_limit.as_ref().unwrap().parse::<u64>().unwrap() == u64::from(certificate_param.max_limit.unwrap())}
                        )).count();
                    assert_eq!(matching_params, 1);
                }
                RewardPot(certificate_param) => {
                    let matching_params = explorer_cert
                        .changes
                        .config_params
                        .iter()
                        .filter(|&config_param| {
                            matches!(config_param, configParam::RewardPot(explorer_param)
                        if explorer_param.reward_pot == certificate_param.to_string())
                        })
                        .count();
                    assert_eq!(matching_params, 1);
                }
                RewardParams(certificate_param) => {
                    let matching_params = explorer_cert.changes.config_params.iter()
                        .filter(|&config_param| matches!(config_param, configParam::RewardParams(explorer_param)
                        if { match &explorer_param.reward_params {
                            ConfigParamOnRewardParamsRewardParams::LinearRewardParams(exp_linear_param) =>
                                {matches!(certificate_param, RewardParams::Linear { constant,ratio,epoch_rate,epoch_start }
                                    if {*constant == exp_linear_param.constant as u64 &&
                                        ratio.numerator == exp_linear_param.ratio.numerator.parse::<u64>().unwrap() &&
                                        u64::from(ratio.denominator) == exp_linear_param.ratio.denominator.parse::<u64>().unwrap() &&
                                        u32::from(*epoch_rate) == exp_linear_param.epoch_rate as u32 &&
                                        *epoch_start == exp_linear_param.epoch_start as u32}) },
                            ConfigParamOnRewardParamsRewardParams::HalvingRewardParams(exp_halving_param) =>
                                {matches!(certificate_param, RewardParams::Halving { constant,ratio,epoch_rate,epoch_start }
                                    if {*constant == exp_halving_param.constant as u64 &&
                                        ratio.numerator == exp_halving_param.ratio.numerator.parse::<u64>().unwrap() &&
                                        u64::from(ratio.denominator) == exp_halving_param.ratio.denominator.parse::<u64>().unwrap() &&
                                        u32::from(*epoch_rate) == exp_halving_param.epoch_rate as u32 &&
                                        *epoch_start == exp_halving_param.epoch_start as u32}) },
                        }})).count();
                    assert_eq!(matching_params, 1);
                }
                PerCertificateFees(certificate_param) => {
                    let matching_params = explorer_cert.changes.config_params.iter()
                        .filter(|&config_param| matches!(config_param, configParam::PerCertificateFee(explorer_param)
                        if {
                            explorer_param.certificate_owner_stake_delegation.unwrap() as u64 == u64::from(certificate_param.certificate_owner_stake_delegation.unwrap()) &&
                            explorer_param.certificate_pool_registration.unwrap() as u64 == u64::from(certificate_param.certificate_pool_registration.unwrap()) &&
                            explorer_param.certificate_stake_delegation.unwrap() as u64 == u64::from(certificate_param.certificate_stake_delegation.unwrap())
                        })).count();
                    assert_eq!(matching_params, 1);
                }
                FeesInTreasury(certificate_param) => {
                    let matching_params = explorer_cert
                        .changes
                        .config_params
                        .iter()
                        .filter(|&config_param| {
                            matches!(config_param, configParam::FeesInTreasury(explorer_param)
                        if explorer_param.fees_in_treasury == *certificate_param)
                        })
                        .count();
                    assert_eq!(matching_params, 1);
                }
                RewardLimitNone => {
                    let matching_params = explorer_cert
                        .changes
                        .config_params
                        .iter()
                        .filter(|&config_param| {
                            matches!(config_param, configParam::RewardLimitNone(explorer_param)
                        if !explorer_param.reward_limit_none)
                        })
                        .count();
                    assert_eq!(matching_params, 1);
                }
                RewardLimitByAbsoluteStake(certificate_param) => {
                    let matching_params = explorer_cert.changes.config_params.iter()
                        .filter(|&config_param| matches!(config_param, configParam::RewardLimitByAbsoluteStake(explorer_param)
                        if explorer_param.reward_limit_by_absolute_stake.numerator.parse::<u64>().unwrap() == certificate_param.numerator &&
                            explorer_param.reward_limit_by_absolute_stake.denominator.parse::<u64>().unwrap() == u64::from(certificate_param.denominator))).count();
                    assert_eq!(matching_params, 1);
                }
                PoolRewardParticipationCapping(certificate_param) => {
                    let matching_params = explorer_cert.changes.config_params.iter()
                        .filter(|&config_param| matches!(config_param, configParam::PoolRewardParticipationCapping(explorer_param)
                        if explorer_param.max as u32 == u32::from(certificate_param.0) &&
                            explorer_param.min as u32 == u32::from(certificate_param.1))).count();
                    assert_eq!(matching_params, 1);
                }
                AddCommitteeId(certificate_param) => {
                    let matching_params = explorer_cert.changes.config_params.iter()
                        .filter(|&config_param| matches!(config_param, configParam::AddCommitteeId(explorer_param)
                        if explorer_param.add_committee_id == certificate_param.public_key().to_string())).count();
                    assert_eq!(matching_params, 1);
                }
                RemoveCommitteeId(certificate_param) => {
                    let matching_params = explorer_cert.changes.config_params.iter()
                        .filter(|&config_param| matches!(config_param, configParam::RemoveCommitteeId(explorer_param)
                        if explorer_param.remove_committee_id == certificate_param.public_key().to_string())).count();
                    assert_eq!(matching_params, 1);
                }
                PerVoteCertificateFees(certificate_param) => {
                    let matching_params = explorer_cert.changes.config_params.iter()
                        .filter(|&config_param| matches!(config_param, configParam::PerVoteCertificateFee(explorer_param)
                        if {explorer_param.certificate_vote_cast.unwrap() as u64 == u64::from(certificate_param.certificate_vote_cast.unwrap()) &&
                            explorer_param.certificate_vote_plan.unwrap() as u64 == u64::from(certificate_param.certificate_vote_plan.unwrap())
                        })).count();
                    assert_eq!(matching_params, 1);
                }
                TransactionMaxExpiryEpochs(certificate_param) => {
                    let matching_params = explorer_cert.changes.config_params.iter()
                        .filter(|&config_param| matches!(config_param, configParam::TransactionMaxExpiryEpochs(explorer_param)
                        if explorer_param.transaction_max_expiry_epochs as u8 == *certificate_param)).count();
                    assert_eq!(matching_params, 1);
                }
                #[cfg(feature = "evm")]
                EvmConfiguration(_) => unimplemented!(),
                #[cfg(feature = "evm")]
                EvmEnvironment(_) => unimplemented!(),
            }
        }
    }

    fn assert_update_vote(
        fragment_cert: Transaction<UpdateVote>,
        explorer_cert: TransactionByIdCertificatesTransactionCertificateOnUpdateVote,
    ) {
        let update_vote_cert = fragment_cert.as_slice().payload().into_payload();
        assert_eq!(
            explorer_cert.proposal_id,
            update_vote_cert.proposal_id().to_string()
        );
        assert_eq!(
            Self::decode_bech32_pk(&explorer_cert.voter_id.id),
            *update_vote_cert.voter_id().as_public_key()
        );
    }

    pub fn assert_epoch_stability_depth(depth: u32, explorer_depth: i64) {
        assert_eq!(depth as u64, explorer_depth as u64);
    }

    pub fn assert_fees(fees: LinearFee, explorer_fees: SettingsSettingsFees) {
        assert_eq!(explorer_fees.certificate as u64, fees.certificate);
        assert_eq!(explorer_fees.coefficient as u64, fees.coefficient);
        assert_eq!(explorer_fees.constant as u64, fees.constant);
        assert_eq!(
            explorer_fees
                .per_certificate_fees
                .certificate_owner_stake_delegation
                .unwrap() as u64,
            u64::from(
                fees.per_certificate_fees
                    .certificate_owner_stake_delegation
                    .unwrap()
            )
        );
        assert_eq!(
            explorer_fees
                .per_certificate_fees
                .certificate_pool_registration
                .unwrap() as u64,
            u64::from(
                fees.per_certificate_fees
                    .certificate_pool_registration
                    .unwrap()
            )
        );
        assert_eq!(
            explorer_fees
                .per_certificate_fees
                .certificate_stake_delegation
                .unwrap() as u64,
            u64::from(
                fees.per_certificate_fees
                    .certificate_stake_delegation
                    .unwrap()
            )
        );
        assert_eq!(
            explorer_fees
                .per_vote_certificate_fees
                .certificate_vote_cast
                .unwrap() as u64,
            u64::from(
                fees.per_vote_certificate_fees
                    .certificate_vote_cast
                    .unwrap()
            )
        );
        assert_eq!(
            explorer_fees
                .per_vote_certificate_fees
                .certificate_vote_plan
                .unwrap() as u64,
            u64::from(
                fees.per_vote_certificate_fees
                    .certificate_vote_plan
                    .unwrap()
            )
        );
    }

    pub fn assert_address(address: Address, explorer_address: AddressAddress) {
        assert_eq!(address.to_string(), explorer_address.id);
    }

    pub fn assert_transactions_address(
        fragment_statuses: HashMap<String, (&Fragment, &FragmentStatus)>,
        explorer_transactions: TransactionsByAddressTipTransactionsByAddress,
    ) {
        if fragment_statuses.is_empty() {
            assert!(explorer_transactions.total_count == 0);
        } else {
            assert_eq!(
                fragment_statuses.len() as i64 + 1,
                explorer_transactions.total_count
            );
        };

        assert!(explorer_transactions.edges.is_some());

        assert_eq!(
            fragment_statuses.len(),
            explorer_transactions.edges.as_ref().unwrap().len()
        );

        for edges in explorer_transactions.edges.unwrap().iter() {
            let node = &edges.as_ref().unwrap().node;
            assert!(fragment_statuses.get(&node.id.to_string()).is_some());
            let fragment_status = fragment_statuses.get(&node.id.to_string()).unwrap().1;
            assert!(
                matches!(fragment_status, FragmentStatus::InABlock { date, block: _ } if
                    date.epoch() == node.blocks[0].date.epoch.id.parse::<u32>().unwrap() && date.slot() == node.blocks[0].date.slot.parse::<u32>().unwrap()
                )
            );
            let fragment = fragment_statuses.get(&node.id.to_string()).unwrap().0;
            assert_eq!(fragment.hash().to_string(), node.id.to_string());
        }
    }

    pub fn assert_vote_plan_by_id(
        explorer_vote_plan: VotePlanByIdVotePlan,
        vote_plan_status: VotePlanStatus,
        proposal_votes: HashMap<String, Vec<(Wallet, Choice)>>,
    ) {
        assert_eq!(explorer_vote_plan.id, vote_plan_status.id.to_string());
        assert_eq!(
            explorer_vote_plan.vote_start.epoch.id,
            vote_plan_status.vote_start.epoch().to_string()
        );
        assert_eq!(
            explorer_vote_plan.vote_start.slot,
            vote_plan_status.vote_start.slot().to_string()
        );
        assert_eq!(
            explorer_vote_plan.vote_end.epoch.id,
            vote_plan_status.vote_end.epoch().to_string()
        );
        assert_eq!(
            explorer_vote_plan.vote_end.slot,
            vote_plan_status.vote_end.slot().to_string()
        );
        assert_eq!(
            explorer_vote_plan.committee_end.epoch.id,
            vote_plan_status.committee_end.epoch().to_string()
        );
        assert_eq!(
            explorer_vote_plan.committee_end.slot,
            vote_plan_status.committee_end.slot().to_string()
        );
        match explorer_vote_plan.payload_type {
            vote_plan_by_id::PayloadType::PUBLIC => assert!(matches!(
                vote_plan_status.payload,
                vote::PayloadType::Public
            )),
            vote_plan_by_id::PayloadType::PRIVATE => assert!(matches!(
                vote_plan_status.payload,
                vote::PayloadType::Private
            )),
            vote_plan_by_id::PayloadType::Other(_) => panic!("Wrong payload type"),
        }

        assert_eq!(
            explorer_vote_plan.proposals.len(),
            vote_plan_status.proposals.len()
        );
        for explorer_proposal in explorer_vote_plan.proposals {
            let vote_proposal_status =
                match vote_plan_status.proposals.iter().position(|proposal| {
                    explorer_proposal.proposal_id == proposal.proposal_id.to_string()
                }) {
                    Some(index) => vote_plan_status.proposals[index].clone(),
                    None => panic!("Proposal id not found"),
                };
            assert_eq!(
                vote_proposal_status.options.start,
                explorer_proposal.options.start as u8
            );
            assert_eq!(
                vote_proposal_status.options.end,
                explorer_proposal.options.end as u8
            );
            match &vote_proposal_status.tally {
                Tally::Public { result } => {
                    if let TallyPublicStatus(explorer_tally_status) =
                        explorer_proposal.tally.unwrap()
                    {
                        assert_eq!(result.results.len(), explorer_tally_status.results.len());
                        let matching_results = result
                            .results
                            .iter()
                            .zip(explorer_tally_status.results.iter())
                            .filter(|&(a, b)| &a.to_string() == b)
                            .count();
                        assert_eq!(matching_results, result.results.len());
                        assert_eq!(result.options.len(), explorer_tally_status.results.len());
                        assert_eq!(
                            result.options.start,
                            explorer_tally_status.options.start as u8
                        );
                        assert_eq!(result.options.end, explorer_tally_status.options.end as u8);
                    } else {
                        panic!("Wrong tally status. Expected Public")
                    }
                }
                Tally::Private { state } => {
                    assert!(explorer_proposal.tally.is_some());
                    if let TallyPrivateStatus(explorer_tally_status) =
                        explorer_proposal.tally.unwrap()
                    {
                        match state {
                            PrivateTallyState::Encrypted { encrypted_tally: _ } => assert!(
                                explorer_tally_status.results.is_none(),
                                "BUG NPG-3369 fixed"
                            ),
                            PrivateTallyState::Decrypted { result } => {
                                let explorer_tally_result = explorer_tally_status.results.unwrap();
                                assert_eq!(result.results.len(), explorer_tally_result.len());
                                let matching_results = result
                                    .results
                                    .iter()
                                    .zip(explorer_tally_result.iter())
                                    .filter(|&(a, b)| &a.to_string() == b)
                                    .count();
                                assert_eq!(matching_results, result.results.len());
                                assert_eq!(result.options.len(), explorer_tally_result.len());
                                assert_eq!(
                                    result.options.start,
                                    explorer_tally_status.options.start as u8
                                );
                                assert_eq!(
                                    result.options.end,
                                    explorer_tally_status.options.end as u8
                                );
                            }
                        }
                    } else {
                        panic!("Wrong tally status. Expected Private")
                    }
                }
            }
            assert_eq!(
                vote_proposal_status.votes_cast,
                explorer_proposal.votes.total_count as usize
            );
            if vote_proposal_status.votes_cast == 0 {
                assert!(explorer_proposal.votes.edges.unwrap().is_empty());
            } else {
                let explorer_votes = explorer_proposal.votes.edges.unwrap();
                assert_eq!(explorer_votes.len(), vote_proposal_status.votes_cast);
                let votes = proposal_votes
                    .get(&vote_proposal_status.proposal_id.to_string())
                    .unwrap();
                for vote in votes {
                    for explorer_vote in &explorer_votes {
                        if vote.0.public_key().to_string()
                            == explorer_vote.as_ref().unwrap().node.address.id
                        {
                            match &explorer_vote.as_ref().unwrap().node.payload {
                                VotePayloadPublicStatus(choice) => {
                                    assert_eq!(choice.choice as u8, vote.1.as_byte())
                                }
                                VotePayloadPrivateStatus(_) => todo!(),
                            }
                        }
                    }
                }
            }
        }
    }

    fn decode_bech32_pk(bech32_public_key: &str) -> PublicKey<Ed25519> {
        let (_, data, _variant) = bech32::decode(bech32_public_key).unwrap();
        let dat = Vec::from_base32(&data).unwrap();
        PublicKey::<Ed25519>::from_binary(&dat).unwrap()
    }
}
