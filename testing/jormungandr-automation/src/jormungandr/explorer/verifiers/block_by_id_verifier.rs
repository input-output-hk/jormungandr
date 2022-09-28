use super::{ExplorerVerifier, VerifierError};
use crate::jormungandr::explorer::data::block_by_id::{PayloadType as expPayloadType, *};
use chain_addr::AddressReadable;
use chain_impl_mockchain::{
    account::DelegationType,
    block::Block,
    certificate::*,
    chaintypes::ConsensusType,
    config::{ConfigParam::*, RewardParams},
    fragment::Fragment,
    transaction::{AccountIdentifier, InputEnum, Transaction},
    vote::PayloadType,
};
use std::num::NonZeroU64;

impl ExplorerVerifier {
    pub fn assert_block_by_id(
        block: Block,
        explorer_block: BlockByIdBlock,
    ) -> Result<(), VerifierError> {
        assert_eq!(explorer_block.id, block.header().id().to_string());
        assert_eq!(
            explorer_block.date.epoch.id.parse::<u32>().unwrap(),
            block.header().block_date().epoch
        );
        assert_eq!(
            explorer_block.date.slot.parse::<u32>().unwrap(),
            block.header().block_date().slot_id
        );
        assert_eq!(
            explorer_block.chain_length,
            block.header().chain_length().to_string()
        );
        assert_eq!(
            explorer_block.previous_block.id,
            block.header().block_parent_hash().to_string()
        );
        match explorer_block.leader {
            Some(leader) => match leader {
                BlockByIdBlockLeader::Pool(leader) => assert_eq!(
                    leader.id,
                    block.header().get_stakepool_id().unwrap().to_string()
                ),
                BlockByIdBlockLeader::BftLeader(leader) => assert_eq!(
                    Self::decode_bech32_pk(leader.id.as_str()),
                    *block.header().get_bft_leader_id().unwrap().as_public_key()
                ),
            },
            None => {
                assert!(block.header().get_stakepool_id().is_none());
                assert!(block.header().get_bft_leader_id().is_none());
            }
        }
        //Logging the fragments in the block
        for f in block.fragments() {
            match f {
                Fragment::Initial(_) => println!("Fragment::Initial hash {}", f.hash()),
                Fragment::OldUtxoDeclaration(_) => {
                    println!("Fragment::OldUtxoDeclaration hash {}", f.hash())
                }
                Fragment::Transaction(_) => {
                    println!("Fragment::Transaction hash {}", f.hash())
                }
                Fragment::OwnerStakeDelegation(_) => {
                    println!("Fragment::OwnerStakeDelegation hash {}", f.hash())
                }
                Fragment::StakeDelegation(_) => {
                    println!("Fragment::StakeDelegation hash {}", f.hash())
                }
                Fragment::PoolRegistration(_) => {
                    println!("Fragment::PoolRegistration hash {}", f.hash())
                }
                Fragment::PoolRetirement(_) => {
                    println!("Fragment::PoolRetirement hash {}", f.hash())
                }
                Fragment::PoolUpdate(_) => {
                    println!("Fragment::PoolUpdate hash {}", f.hash())
                }
                Fragment::UpdateProposal(_) => {
                    println!("Fragment::UpdateProposal hash {}", f.hash())
                }
                Fragment::UpdateVote(_) => {
                    println!("Fragment::UpdateVote hash {}", f.hash())
                }
                Fragment::VotePlan(_) => {
                    println!("Fragment::VotePlan hash {}", f.hash())
                }
                Fragment::VoteCast(_) => {
                    println!("Fragment::VoteCast hash {}", f.hash())
                }
                Fragment::VoteTally(_) => {
                    println!("Fragment::VoteTall hash {}", f.hash())
                }
                Fragment::MintToken(_) => {
                    println!("Fragment::MintToken hash {}", f.hash())
                }
                Fragment::Evm(_) => println!("Fragment::Evm hash {}", f.hash()),
                Fragment::EvmMapping(_) => {
                    println!("Fragment::EvmMapping hash {}", f.hash())
                }
            }
        }

        assert_eq!(
            explorer_block.transactions.total_count,
            block.contents().len() as i64
        );
        let mut matching_fragments_count = 0;

        if !block.contents().is_empty() {
            for fragment in block.fragments() {
                for edge in &explorer_block.transactions.edges {
                    let explorer_transaction = &edge.node;
                    if fragment.hash().to_string() == explorer_transaction.id {
                        matching_fragments_count += 1;
                        match &explorer_transaction.certificate {
                            None => {
                                if let Fragment::Transaction(fragment_transaction) = fragment {
                                    Self::assert_block_transaction_param(
                                        fragment_transaction,
                                        explorer_transaction,
                                    )
                                    .unwrap();
                                } else if let Fragment::Initial(_config_params) = fragment {
                                    //Not implemented because of the bug //NPG-3274
                                    return Err(VerifierError::Unimplemented);
                                } else {
                                    return Err(VerifierError::InvalidCertificate {
                                        received: "Transaction".to_string(),
                                    });
                                };
                            }
                            Some(certificate) => {
                                match certificate{
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::StakeDelegation(
                                            explorer_cert,
                                        ) => {
                                            if let Fragment::StakeDelegation(fragment_cert) = fragment {
                                                Self::assert_block_transaction_param(
                                                    &fragment_cert.clone(),
                                                    explorer_transaction,
                                                )
                                                .unwrap();
                                                Self::assert_block_stake_delegation(fragment_cert, explorer_cert)
                                                    .unwrap();
                                            } else {
                                                return Err(VerifierError::InvalidCertificate {
                                                    received: "StakeDelegation".to_string(),
                                                });
                                            }
                                        }
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::OwnerStakeDelegation(
                                            explorer_cert,
                                        ) => {
                                            if let Fragment::OwnerStakeDelegation(fragment_cert) = fragment {
                                                Self::assert_block_transaction_param(
                                                    &fragment_cert.clone(),
                                                    explorer_transaction,
                                                )
                                                .unwrap();
                                               Self::assert_block_owner_delegation(fragment_cert, explorer_cert)
                                                    .unwrap();
                                            } else {
                                                return Err(VerifierError::InvalidCertificate {
                                                    received: "OwnerStakeDelegation".to_string(),
                                                });
                                            }
                                        }
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::PoolRegistration(
                                            explorer_cert,
                                        ) => {
                                            if let Fragment::PoolRegistration(fragment_cert) = fragment {
                                                Self::assert_block_transaction_param(
                                                    &fragment_cert.clone(),
                                                    explorer_transaction,
                                                )
                                                .unwrap();
                                              Self::assert_block_pool_registration(fragment_cert, explorer_cert);
                                            } else {
                                               return Err(VerifierError::InvalidCertificate {
                                                    received: "PoolRegistration".to_string(),
                                                });
                                            }
                                        }
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::PoolRetirement(
                                            explorer_cert,
                                        ) => {
                                            if let Fragment::PoolRetirement(fragment_cert) = fragment {
                                                Self::assert_block_transaction_param(
                                                    &fragment_cert.clone(),
                                                    explorer_transaction,
                                                )
                                                .unwrap();
                                                Self::assert_block_pool_retirement(fragment_cert, explorer_cert);
                                            } else {
                                               return Err(VerifierError::InvalidCertificate {
                                                    received: "PoolRetirement".to_string(),
                                                });
                                            }
                                        }
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::PoolUpdate(explorer_cert) => {
                                            if let Fragment::PoolUpdate(fragment_cert) = fragment {
                                                Self::assert_block_transaction_param(
                                                    &fragment_cert.clone(),
                                                    explorer_transaction,
                                                )
                                                .unwrap();
                                               Self::assert_block_pool_update(fragment_cert, explorer_cert);
                                            } else {
                                               return Err(VerifierError::InvalidCertificate {
                                                    received: "PoolUpdate".to_string(),
                                                });
                                            }
                                        }
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::VotePlan(explorer_cert) => {
                                            if let Fragment::VotePlan(fragment_cert) = fragment {
                                                Self::assert_block_transaction_param(
                                                    &fragment_cert.clone(),
                                                    explorer_transaction,
                                                )
                                                .unwrap();
                                                Self::assert_block_vote_plan(fragment_cert, explorer_cert);
                                            } else {
                                               return Err(VerifierError::InvalidCertificate {
                                                    received: "VotePlan".to_string(),
                                                });
                                            }
                                        }
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::VoteCast(explorer_cert) => {
                                            if let Fragment::VoteCast(fragment_cert) = fragment {
                                                Self::assert_block_transaction_param(
                                                    &fragment_cert.clone(),
                                                    explorer_transaction,
                                                )
                                                .unwrap();
                                               Self::assert_block_vote_cast(fragment_cert, explorer_cert);
                                            } else {
                                               return Err(VerifierError::InvalidCertificate {
                                                    received: "VoteCast".to_string(),
                                                });
                                            }
                                        }
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::VoteTally(explorer_cert) => {
                                            if let Fragment::VoteTally(fragment_cert) = fragment {
                                                Self::assert_block_transaction_param(
                                                    &fragment_cert.clone(),
                                                    explorer_transaction,
                                                )
                                                .unwrap();
                                              Self::assert_block_vote_tally(fragment_cert, explorer_cert);
                                            } else {
                                               return Err(VerifierError::InvalidCertificate {
                                                    received: "VoteTally".to_string(),
                                                });
                                            }
                                        }
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::UpdateProposal(
                                            explorer_cert,
                                        ) => {
                                            if let Fragment::UpdateProposal(fragment_cert) = fragment {
                                                Self::assert_block_transaction_param(
                                                    &fragment_cert.clone(),
                                                    explorer_transaction,
                                                )
                                                .unwrap();
                                             Self::assert_block_update_proposal(fragment_cert, explorer_cert);
                                            } else {
                                             return  Err(VerifierError::InvalidCertificate {
                                                    received: "UpdateProposal".to_string(),
                                                });
                                            }
                                        }
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::UpdateVote(explorer_cert) => {
                                            if let Fragment::UpdateVote(fragment_cert) = fragment {
                                                Self::assert_block_transaction_param(
                                                    &fragment_cert.clone(),
                                                    explorer_transaction,
                                                )
                                                .unwrap();
                                            Self::assert_block_update_vote(fragment_cert, explorer_cert);
                                            } else {
                                              return Err(VerifierError::InvalidCertificate {
                                                    received: "UpdateVote".to_string(),
                                                });
                                            }
                                        }
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::MintToken(_) => {
                                           return Err(VerifierError::InvalidCertificate {
                                                received: "MintToken can be only in block0".to_string(),
                                            });
                                        }
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::EvmMapping(_) => {
                                            //Not implemented because of the bug EAS-238
                                           return Err(VerifierError::Unimplemented);
                                        }
                                    }
                            }
                        }
                    }
                }
            }
            assert_eq!(matching_fragments_count, block.contents().len() as i32);
        }
        Ok(())
    }

    fn assert_block_transaction_param<P>(
        fragment_transaction: &Transaction<P>,
        explorer_transaction: &BlockByIdBlockTransactionsEdgesNode,
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

    fn assert_block_pool_registration(
        fragment_cert: &Transaction<PoolRegistration>,
        explorer_cert: &BlockByIdBlockTransactionsEdgesNodeCertificateOnPoolRegistration,
    ) {
        let pool_cert = fragment_cert.as_slice().payload().into_payload();

        assert_eq!(pool_cert.to_id().to_string(), explorer_cert.pool.id);
        assert_eq!(
            u64::from(pool_cert.start_validity),
            explorer_cert.start_validity.parse::<u64>().unwrap()
        );
        if pool_cert.reward_account.is_some() {
            if let AccountIdentifier::Single(id) = pool_cert.reward_account.as_ref().unwrap() {
                assert_eq!(
                    id.to_string(),
                    explorer_cert.reward_account.as_ref().unwrap().id
                );
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
                    .as_ref()
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
    }

    fn assert_block_stake_delegation(
        fragment_cert: &Transaction<StakeDelegation>,
        explorer_cert: &BlockByIdBlockTransactionsEdgesNodeCertificateOnStakeDelegation,
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
    fn assert_block_owner_delegation(
        fragment_cert: &Transaction<OwnerStakeDelegation>,
        explorer_cert: &BlockByIdBlockTransactionsEdgesNodeCertificateOnOwnerStakeDelegation,
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

    fn assert_block_pool_retirement(
        fragment_cert: &Transaction<PoolRetirement>,
        explorer_cert: &BlockByIdBlockTransactionsEdgesNodeCertificateOnPoolRetirement,
    ) {
        let ret_cert = fragment_cert.as_slice().payload().into_payload();
        assert_eq!(ret_cert.pool_id.to_string(), explorer_cert.pool_id);
        assert_eq!(
            u64::from(ret_cert.retirement_time),
            explorer_cert.retirement_time.parse::<u64>().unwrap()
        );
    }

    fn assert_block_pool_update(
        fragment_cert: &Transaction<PoolUpdate>,
        explorer_cert: &BlockByIdBlockTransactionsEdgesNodeCertificateOnPoolUpdate,
    ) {
        let update_cert = fragment_cert.as_slice().payload().into_payload();
        assert_eq!(update_cert.pool_id.to_string(), explorer_cert.pool_id);
        assert_eq!(
            u64::from(update_cert.new_pool_reg.start_validity),
            explorer_cert.start_validity.parse::<u64>().unwrap()
        );
    }

    fn assert_block_vote_plan(
        fragment_cert: &Transaction<VotePlan>,
        explorer_cert: &BlockByIdBlockTransactionsEdgesNodeCertificateOnVotePlan,
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

    fn assert_block_vote_cast(
        fragment_cert: &Transaction<VoteCast>,
        explorer_cert: &BlockByIdBlockTransactionsEdgesNodeCertificateOnVoteCast,
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

    fn assert_block_vote_tally(
        fragment_cert: &Transaction<VoteTally>,
        explorer_cert: &BlockByIdBlockTransactionsEdgesNodeCertificateOnVoteTally,
    ) {
        let vote_tally_cert = fragment_cert.as_slice().payload().into_payload();
        assert_eq!(explorer_cert.vote_plan, vote_tally_cert.id().to_string());
    }

    fn assert_block_update_proposal(
        fragment_cert: &Transaction<UpdateProposal>,
        explorer_cert: &BlockByIdBlockTransactionsEdgesNodeCertificateOnUpdateProposal,
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

    fn assert_block_update_vote(
        fragment_cert: &Transaction<UpdateVote>,
        explorer_cert: &BlockByIdBlockTransactionsEdgesNodeCertificateOnUpdateVote,
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
}
