use crate::jcli_lib::utils::io;
use crate::jcli_lib::utils::OutputFormat;
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_lib::interfaces::VotePlanId;
use jormungandr_lib::interfaces::{PrivateTallyState, Tally, VotePlanStatus, VoteProposalStatus};
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::ops::Range;
use std::path::PathBuf;
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

fn merge_voteplans(mut voteplans: Vec<VotePlanStatus>) -> Result<Vec<MergedVotePlan>, Error> {
    let mut results = Vec::with_capacity(voteplans.len() / 2);

    while !voteplans.is_empty() {
        let vp1 = voteplans.swap_remove(0);

        let mut ids = BTreeSet::from_iter([vp1.id]);

        let mut p1: BTreeMap<Hash, MergedVoteProposalStatus> = vp1
            .proposals
            .into_iter()
            .map(|vps| {
                (
                    vps.proposal_id,
                    MergedVoteProposalStatus {
                        proposal_id: vps.proposal_id,
                        options: vps.options,
                        tally: vps.tally,
                        votes_cast: vps.votes_cast,
                    },
                )
            })
            .collect();

        // indexes of the proposals that have the same external proposal set as vp1
        let mut matches = voteplans
            .iter()
            .enumerate()
            .filter_map(|(i, vp2)| {
                if vp2.proposals.len() == p1.len()
                    && vp2
                        .proposals
                        .iter()
                        .all(|p| p1.get(&p.proposal_id).is_some())
                {
                    Some(i)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for i in &matches {
            let vp2 = &voteplans[*i];
            ids.insert(vp2.id);

            let p2: BTreeMap<Hash, &VoteProposalStatus> = vp2
                .proposals
                .iter()
                .map(|vps| (vps.proposal_id, vps))
                .collect();

            for (mut ps1, ps2) in p1.values_mut().zip(p2.into_iter().map(|(_, v)| v)) {
                ps1.votes_cast += ps2.votes_cast;
                ps1.tally = match (&ps1.tally, &ps2.tally) {
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

        // already merged proposals shouldn't be considered again
        //
        // reverse is necessary because the last index can be in the list
        matches.reverse();
        for i in matches {
            voteplans.swap_remove(i);
        }

        // FIXME: I'm not actually sure if this should be the correct behavior.
        //
        // On one hand, a voteplan without matches can't be merged.
        //
        // On the other, it can be considered as is trivially merged.
        if ids.len() > 1 {
            results.push(MergedVotePlan {
                ids,
                proposals: p1.into_iter().map(|(_, v)| v).collect(),
            });
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use chain_core::property::FromStr;
    use chain_impl_mockchain::{
        tokens::identifier::{self, TokenIdentifier},
        vote::PayloadType,
    };
    use jormungandr_lib::interfaces::{BlockDate, TallyResult, VotePlanId};

    use super::*;

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

        let result = merge_voteplans(voteplans).unwrap();

        assert_eq!(result.len(), 2);

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
