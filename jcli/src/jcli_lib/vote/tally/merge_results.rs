use crate::jcli_lib::utils::{io, OutputFormat};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{PrivateTallyState, Tally, VotePlanId, VotePlanStatus},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    ops::Range,
    path::PathBuf,
};
use structopt::StructOpt;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("voteplan should be already decrypted before merging")]
    VotePlanEncrypted,
    #[error("voteplans have different privacy type")]
    PrivacyMismatch,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct MergeVotePlan {
    /// The path to json-encoded list of voteplans to merge. If this parameter is not specified, it
    /// will be read from the standard input. Voteplans must be already decrypted before merging.
    /// Two voteplans in the list will be merged if they have ALL the same proposals according to
    /// the proposal (external) id.
    #[structopt(long)]
    vote_plans: Option<PathBuf>,
    #[structopt(flatten)]
    output_format: OutputFormat,
}

impl MergeVotePlan {
    pub fn exec(&self) -> Result<(), super::Error> {
        let voteplans: Vec<VotePlanStatus> =
            serde_json::from_reader(io::open_file_read(&self.vote_plans)?)?;

        let results = merge_voteplans(voteplans)?;

        let output = self
            .output_format
            .format_json(serde_json::to_value(results)?)?;
        println!("{}", output);

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct MergedVotePlan {
    pub ids: BTreeSet<VotePlanId>,
    pub proposals: Vec<MergedVoteProposalStatus>,
}

/// like VoteProposalStatus but without the index field, since the proposals can be in different
/// indexes with the current implementation.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MergedVoteProposalStatus {
    pub proposal_id: Hash,
    pub options: Range<u8>,
    pub tally: Tally,
    pub votes_cast: usize,
}

fn merge_voteplans(voteplans: Vec<VotePlanStatus>) -> Result<Vec<MergedVotePlan>, Error> {
    let mut group_by_proposals: HashMap<Vec<Hash>, Vec<VotePlanStatus>> = HashMap::new();

    for mut voteplan in voteplans.into_iter() {
        // this matters not only to have a normal form for the keys of the hashmap, but also to be
        // able to zip and merge later
        voteplan.proposals.sort_by_key(|p| p.proposal_id);

        let ids = voteplan
            .proposals
            .iter()
            .map(|p| p.proposal_id)
            .collect::<Vec<_>>();

        group_by_proposals.entry(ids).or_default().push(voteplan);
    }

    group_by_proposals
        .into_iter()
        .map(|(_key, mut group)| {
            let ids = group.iter().map(|group| group.id).collect();

            let mut proposals = group
                .pop()
                .map(|p| {
                    p.proposals
                        .into_iter()
                        .map(|p| MergedVoteProposalStatus {
                            proposal_id: p.proposal_id,
                            options: p.options,
                            tally: p.tally,
                            votes_cast: p.votes_cast,
                        })
                        .collect::<Vec<_>>()
                })
                // there has to be at least one entry, since  the key comes from the value, so this
                // can't panic.
                .unwrap();

            for vps in group {
                for (a, b) in proposals.iter_mut().zip(vps.proposals.iter()) {
                    a.votes_cast += b.votes_cast;

                    a.tally = match (&a.tally, &b.tally) {
                        (Tally::Public { result: result1 }, Tally::Public { result: result2 }) => {
                            Tally::Public {
                                result: result1.merge(result2),
                            }
                        }
                        (
                            Tally::Private {
                                state: PrivateTallyState::Decrypted { result: result1 },
                            },
                            Tally::Private {
                                state: PrivateTallyState::Decrypted { result: result2 },
                            },
                        ) => Tally::Private {
                            state: PrivateTallyState::Decrypted {
                                result: result1.merge(result2),
                            },
                        },
                        (Tally::Public { result: _ }, Tally::Private { state: _ })
                        | (Tally::Private { state: _ }, Tally::Public { result: _ }) => {
                            return Err(Error::PrivacyMismatch);
                        }
                        (Tally::Private { state: _ }, Tally::Private { state: _ }) => {
                            return Err(Error::VotePlanEncrypted)
                        }
                    };
                }
            }

            Ok(MergedVotePlan { ids, proposals })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chain_core::property::FromStr;
    use chain_impl_mockchain::{
        tokens::identifier::{self, TokenIdentifier},
        vote::PayloadType,
    };
    use jormungandr_lib::interfaces::{BlockDate, TallyResult, VotePlanId, VoteProposalStatus};

    fn gen_voteplan_status(
        token: TokenIdentifier,
        results: [u64; 2],
        votes_cast: usize,
        proposal_id: Hash,
        id: VotePlanId,
    ) -> VotePlanStatus {
        VotePlanStatus {
            id,
            payload: PayloadType::Private,
            vote_start: BlockDate::new(0, 0),
            vote_end: BlockDate::new(0, 1),
            committee_end: BlockDate::new(0, 2),
            committee_member_keys: vec![],
            proposals: vec![VoteProposalStatus {
                index: 0,
                proposal_id,
                options: 0..2,
                tally: Tally::Private {
                    state: PrivateTallyState::Decrypted {
                        result: TallyResult {
                            results: results.try_into().unwrap(),
                            options: 0..2,
                        },
                    },
                },
                votes_cast,
            }],
            voting_token: token.into(),
        }
    }

    #[test]
    fn merge_decrypted_voteplans() {
        let mut voteplans = Vec::new();

        let voting_token1 = TokenIdentifier::from_str(
            "00000000000000000000000000000000000000000000000000000000.00000000",
        )
        .unwrap();

        let voting_token2 = identifier::TokenIdentifier::from_str(
            "11111111111111111111111111111111111111111111111111111111.00000000",
        )
        .unwrap();

        let voting_token3 = identifier::TokenIdentifier::from_str(
            "22222222222222222222222222222222222222222222222222222222.00000000",
        )
        .unwrap();

        let voting_token4 = identifier::TokenIdentifier::from_str(
            "33333333333333333333333333333333333333333333333333333333.00000000",
        )
        .unwrap();

        let voteplan1 = gen_voteplan_status(
            voting_token1.clone(),
            [1, 1],
            2,
            Hash::from([1u8; 32]),
            VotePlanId::from([1u8; 32]),
        );
        voteplans.push(voteplan1.clone());

        let voteplan2 = gen_voteplan_status(
            voting_token2.clone(),
            [1, 1],
            2,
            Hash::from([1u8; 32]),
            VotePlanId::from([2u8; 32]),
        );
        voteplans.push(voteplan2.clone());

        let voteplan3 = gen_voteplan_status(
            voting_token1,
            [1, 10],
            3,
            Hash::from([2u8; 32]),
            VotePlanId::from([3u8; 32]),
        );
        voteplans.push(voteplan3.clone());

        let voteplan4 = gen_voteplan_status(
            voting_token2,
            [2, 8],
            4,
            Hash::from([2u8; 32]),
            VotePlanId::from([4u8; 32]),
        );
        voteplans.push(voteplan4.clone());

        let voteplan5 = gen_voteplan_status(
            voting_token3,
            [1, 1],
            2,
            Hash::from([2u8; 32]),
            VotePlanId::from([5u8; 32]),
        );
        voteplans.push(voteplan5.clone());

        // standalone voteplan, should be ignored
        let voteplan6 = gen_voteplan_status(
            voting_token4,
            [1, 0],
            1,
            Hash::from([3u8; 32]),
            VotePlanId::from([6u8; 32]),
        );
        voteplans.push(voteplan6);

        let mut result = merge_voteplans(voteplans).unwrap();

        result.sort_by_key(|r| r.ids.clone());

        assert_eq!(result.len(), 3);

        match &result[0].proposals[0].tally {
            Tally::Private {
                state:
                    PrivateTallyState::Decrypted {
                        result:
                            TallyResult {
                                results,
                                options: _,
                            },
                    },
            } => {
                assert_eq!(results.clone(), vec![2, 2]);
            }
            _ => unreachable!(),
        }

        match &result[1].proposals[0].tally {
            Tally::Private {
                state:
                    PrivateTallyState::Decrypted {
                        result:
                            TallyResult {
                                results,
                                options: _,
                            },
                    },
            } => {
                assert_eq!(results.clone(), vec![4, 19]);
            }
            _ => unreachable!(),
        }

        assert_eq!(result[0].proposals[0].votes_cast, 4);
        assert_eq!(result[1].proposals[0].votes_cast, 3 + 4 + 2);

        assert_eq!(
            result[0].ids,
            BTreeSet::from_iter([voteplan1.id, voteplan2.id])
        );
        assert_eq!(
            result[1].ids,
            BTreeSet::from_iter([voteplan3.id, voteplan4.id, voteplan5.id])
        );
    }
}
