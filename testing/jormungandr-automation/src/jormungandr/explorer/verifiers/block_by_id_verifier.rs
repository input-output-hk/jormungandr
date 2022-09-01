use super::ExplorerVerifier;
use crate::jormungandr::explorer::data::{
    block_by_id::{
        BlockByIdBlock, BlockByIdBlockLeader, BlockByIdBlockTransactionsEdgesNodeCertificate,
    },
    transaction_by_id_certificates::PayloadType as expPayloadType,
};
use bech32::FromBase32;
use chain_addr::AddressReadable;
use chain_core::property::HasHeader;
use chain_crypto::{Ed25519, PublicKey};
use chain_impl_mockchain::{
    account::DelegationType,
    block::Block,
    certificate::*,
    chaintypes::ConsensusType,
    config::{ConfigParam::*, RewardParams},
    fee::LinearFee,
    fragment::Fragment,
    transaction::{AccountIdentifier, InputEnum, Transaction},
    vote::PayloadType,
};
use std::num::NonZeroU64;

impl ExplorerVerifier {
    pub fn assert_block(block: Block, explorer_block: BlockByIdBlock) {
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
        /*  match explorer_block.leader {
            Some(leader) => match leader {
                BlockByIdBlockLeader::Pool(leader) => assert_eq!(
                    leader.id,
                    block.header().get_stakepool_id().unwrap().to_string()
                ),
                BlockByIdBlockLeader::BftLeader(leader) => assert_eq!(
                    leader.id,
                    block
                        .header()
                        .get_bft_leader_id()
                        .unwrap()
                        .as_public_key()
                        .to_string()
                ),
            },
            None => {
                assert!(block.header().get_stakepool_id().is_none());
                assert!(block.header().get_bft_leader_id().is_none());
            }
        }*/
        for b in block.fragments() {
            println!("FRRR {}", b.hash().to_string());
            match b {
                Fragment::Initial(_) => println!("1"),
                Fragment::OldUtxoDeclaration(_) => println!("2"),
                Fragment::Transaction(_) => println!("3"),
                Fragment::OwnerStakeDelegation(_) => println!("4"),
                Fragment::StakeDelegation(_) => println!("5"),
                Fragment::PoolRegistration(_) => println!("6"),
                Fragment::PoolRetirement(_) => println!("7"),
                Fragment::PoolUpdate(_) => println!("8"),
                Fragment::UpdateProposal(_) => println!("9"),
                Fragment::UpdateVote(_) => println!("10"),
                Fragment::VotePlan(_) => println!("11"),
                Fragment::VoteCast(_) => println!("12"),
                Fragment::VoteTally(_) => println!("13"),
                Fragment::MintToken(_) => println!("14"),
                Fragment::Evm(_) => println!("15"),
                Fragment::EvmMapping(_) => println!("16"),
            }
        }
        for a in explorer_block.transactions.edges.as_ref().unwrap() {
            println!("XXX {}", a.as_ref().unwrap().node.id);
        }

        assert_eq!(
            explorer_block.transactions.total_count,
            block.contents().len() as i64
        );
        let mut matching_fragments_count = 0;

        if !block.contents().is_empty() {
            for fragment in block.fragments() {
                for edge in explorer_block.transactions.edges.as_ref().unwrap() {
                    let explorer_transaction = &edge.as_ref().unwrap().node;
                    if fragment.hash().to_string() == explorer_transaction.id {
                        matching_fragments_count += 1;

                        match &explorer_transaction.certificate {
                                None => {
                                    if let Fragment::Transaction(fragment_transaction) = fragment {
                                        //     Self::assert_transaction_params(fragment_transaction, explorer_transaction)
                                        //         .unwrap();
                                        //     Ok(())

                                    } else if let Fragment::Initial(config_params) = fragment {
                                            //  UNIMPLEMENTED
                                    } else {
                                             //Err(VerifierError::InvalidCertificate {
                                             //    received: "Transaction".to_string(),
                                             //})

                                         }
                                },
                                Some(certificate) => {
                                    match certificate{
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::StakeDelegation(_) => todo!(),
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::OwnerStakeDelegation(_) => todo!(),
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::PoolRegistration(_) => todo!(),
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::PoolRetirement(_) => todo!(),
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::PoolUpdate(_) => todo!(),
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::VotePlan(_) => todo!(),
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::VoteCast(_) => todo!(),
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::VoteTally(_) => todo!(),
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::UpdateProposal(_) => todo!(),
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::UpdateVote(_) => todo!(),
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::MintToken(_) => todo!(),
                                        BlockByIdBlockTransactionsEdgesNodeCertificate::EvmMapping(_) => todo!(),
                                    }
                                },
                            }
                    }
                }
            }
            assert_eq!(matching_fragments_count, block.contents().len() as i32);
        }
    }
}
