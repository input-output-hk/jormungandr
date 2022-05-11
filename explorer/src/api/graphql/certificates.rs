use super::{
    config_param::ConfigParams,
    error::ApiError,
    extract_context,
    scalars::{PayloadType, PoolId, PublicKey, TimeOffsetSeconds, VotePlanId},
    Address, BftLeader, BlockDate, ExplorerAddress, Pool, Proposal, TaxType,
};
use async_graphql::{Context, FieldResult, Object, Union};
use chain_impl_mockchain::certificate;

// interface for grouping certificates as a graphl union
#[derive(Union)]
pub enum Certificate {
    StakeDelegation(StakeDelegation),
    OwnerStakeDelegation(OwnerStakeDelegation),
    PoolRegistration(PoolRegistration),
    PoolRetirement(PoolRetirement),
    PoolUpdate(PoolUpdate),
    VotePlan(VotePlan),
    VoteCast(VoteCast),
    VoteTally(VoteTally),
    UpdateProposal(UpdateProposal),
    UpdateVote(UpdateVote),
    MintToken(MintToken),
    EvmMapping(EvmMapping),
}

pub struct StakeDelegation(certificate::StakeDelegation);

pub struct PoolRegistration(certificate::PoolRegistration);

pub struct OwnerStakeDelegation(certificate::OwnerStakeDelegation);

/// Retirement info for a pool
pub struct PoolRetirement(certificate::PoolRetirement);

pub struct PoolUpdate(certificate::PoolUpdate);

pub struct VotePlan(certificate::VotePlan);

pub struct VoteCast(certificate::VoteCast);

pub struct VoteTally(certificate::VoteTally);

pub struct UpdateProposal(certificate::UpdateProposal);

pub struct UpdateVote(certificate::UpdateVote);

pub struct MintToken(certificate::MintToken);

pub struct EvmMapping(certificate::EvmMapping);

#[Object]
impl StakeDelegation {
    // FIXME: Maybe a new Account type would be better?
    pub async fn account(&self, context: &Context<'_>) -> FieldResult<Address> {
        let discrimination = extract_context(context).db.blockchain_config.discrimination;
        self.0
            .account_id
            .to_single_account()
            .ok_or_else(||
                // TODO: Multisig address?
                ApiError::Unimplemented.into())
            .map(|single| {
                chain_addr::Address(discrimination, chain_addr::Kind::Account(single.into()))
            })
            .map(|addr| Address::from(&ExplorerAddress::New(addr)))
    }

    pub async fn pools(&self) -> Vec<Pool> {
        use chain_impl_mockchain::account::DelegationType;

        match self.0.get_delegation_type() {
            DelegationType::NonDelegated => vec![],
            DelegationType::Full(id) => vec![Pool::from_valid_id(id.clone())],
            DelegationType::Ratio(delegation_ratio) => delegation_ratio
                .pools()
                .iter()
                .cloned()
                .map(|(p, _)| Pool::from_valid_id(p))
                .collect(),
        }
    }
}

#[Object]
impl PoolRegistration {
    pub async fn pool(&self) -> Pool {
        Pool::from_valid_id(self.0.to_id())
    }

    /// Beginning of validity for this pool, this is used
    /// to keep track of the period of the expected key and the expiry
    pub async fn start_validity(&self) -> TimeOffsetSeconds {
        self.0.start_validity.into()
    }

    /// Management threshold for owners, this need to be <= #owners and > 0
    pub async fn management_threshold(&self) -> i32 {
        // XXX: u8 fits in i32, but maybe some kind of custom scalar is better?
        self.0.management_threshold().into()
    }

    /// Owners of this pool
    pub async fn owners(&self) -> Vec<PublicKey> {
        self.0.owners.iter().map(PublicKey::from).collect()
    }

    pub async fn operators(&self) -> Vec<PublicKey> {
        self.0.operators.iter().map(PublicKey::from).collect()
    }

    pub async fn rewards(&self) -> TaxType {
        TaxType(self.0.rewards)
    }

    /// Reward account
    pub async fn reward_account(&self, context: &Context<'_>) -> Option<Address> {
        use chain_impl_mockchain::transaction::AccountIdentifier;
        let discrimination = extract_context(context).db.blockchain_config.discrimination;

        // FIXME: Move this transformation to a point earlier

        self.0
            .reward_account
            .clone()
            .map(|acc_id| match acc_id {
                AccountIdentifier::Single(d) => ExplorerAddress::New(chain_addr::Address(
                    discrimination,
                    chain_addr::Kind::Account(d.into()),
                )),
                AccountIdentifier::Multi(d) => {
                    let mut bytes = [0u8; 32];
                    bytes.copy_from_slice(&d.as_ref()[0..32]);
                    ExplorerAddress::New(chain_addr::Address(
                        discrimination,
                        chain_addr::Kind::Multisig(bytes),
                    ))
                }
            })
            .map(|explorer_address| Address {
                id: explorer_address,
            })
    }

    // Genesis Praos keys
    // pub keys: GenesisPraosLeader,
}

#[Object]
impl OwnerStakeDelegation {
    async fn pools(&self) -> Vec<Pool> {
        use chain_impl_mockchain::account::DelegationType;

        match self.0.get_delegation_type() {
            DelegationType::NonDelegated => vec![],
            DelegationType::Full(id) => vec![Pool::from_valid_id(id.clone())],
            DelegationType::Ratio(delegation_ratio) => delegation_ratio
                .pools()
                .iter()
                .cloned()
                .map(|(p, _)| Pool::from_valid_id(p))
                .collect(),
        }
    }
}

#[Object]
impl PoolRetirement {
    pub async fn pool_id(&self) -> PoolId {
        PoolId(self.0.pool_id.clone())
    }

    pub async fn retirement_time(&self) -> TimeOffsetSeconds {
        self.0.retirement_time.into()
    }
}

#[Object]
impl PoolUpdate {
    pub async fn pool_id(&self) -> PoolId {
        PoolId(self.0.pool_id.clone())
    }

    pub async fn start_validity(&self) -> TimeOffsetSeconds {
        self.0.new_pool_reg.start_validity.into()
    }

    // TODO: Previous keys?
    // TODO: Updated keys?
}

#[Object]
impl VotePlan {
    /// the vote start validity
    pub async fn vote_start(&self) -> BlockDate {
        self.0.vote_start().into()
    }

    /// the duration within which it is possible to vote for one of the proposals
    /// of this voting plan.
    pub async fn vote_end(&self) -> BlockDate {
        self.0.vote_end().into()
    }

    /// the committee duration is the time allocated to the committee to open
    /// the ballots and publish the results on chain
    pub async fn committee_end(&self) -> BlockDate {
        self.0.committee_end().into()
    }

    pub async fn payload_type(&self) -> PayloadType {
        self.0.payload_type().into()
    }

    /// the proposals to vote for
    pub async fn proposals(&self) -> Vec<Proposal> {
        self.0.proposals().iter().cloned().map(Proposal).collect()
    }
}

#[Object]
impl VoteCast {
    pub async fn vote_plan(&self) -> VotePlanId {
        self.0.vote_plan().clone().into()
    }

    pub async fn proposal_index(&self) -> i32 {
        self.0.proposal_index() as i32
    }
}

#[Object]
impl VoteTally {
    pub async fn vote_plan(&self) -> VotePlanId {
        self.0.id().clone().into()
    }
}

#[Object]
impl UpdateProposal {
    pub async fn changes(&self) -> ConfigParams {
        self.0.changes().into()
    }

    pub async fn proposer_id(&self) -> BftLeader {
        self.0.proposer_id().clone().into()
    }
}

#[Object]
impl UpdateVote {
    pub async fn proposal_id(&self) -> String {
        format!("{}", self.0.proposal_id())
    }

    pub async fn voter_id(&self) -> BftLeader {
        self.0.voter_id().clone().into()
    }
}

#[Object]
impl MintToken {
    pub async fn name(&self) -> String {
        format!("{:?}", self.0.name)
    }
}

#[Object]
impl EvmMapping {
    pub async fn address(&self) -> String {
        format!("{:?}", self.0)
    }
}

/*------------------------------*/
/*------- Conversions ---------*/
/*----------------------------*/

impl From<chain_impl_mockchain::certificate::Certificate> for Certificate {
    fn from(original: chain_impl_mockchain::certificate::Certificate) -> Certificate {
        match original {
            certificate::Certificate::StakeDelegation(c) => {
                Certificate::StakeDelegation(StakeDelegation(c))
            }
            certificate::Certificate::OwnerStakeDelegation(c) => {
                Certificate::OwnerStakeDelegation(OwnerStakeDelegation(c))
            }
            certificate::Certificate::PoolRegistration(c) => {
                Certificate::PoolRegistration(PoolRegistration(c))
            }
            certificate::Certificate::PoolRetirement(c) => {
                Certificate::PoolRetirement(PoolRetirement(c))
            }
            certificate::Certificate::PoolUpdate(c) => Certificate::PoolUpdate(PoolUpdate(c)),
            certificate::Certificate::VotePlan(c) => Certificate::VotePlan(VotePlan(c)),
            certificate::Certificate::VoteCast(c) => Certificate::VoteCast(VoteCast(c)),
            certificate::Certificate::VoteTally(c) => Certificate::VoteTally(VoteTally(c)),
            certificate::Certificate::UpdateProposal(c) => {
                Certificate::UpdateProposal(UpdateProposal(c))
            }
            certificate::Certificate::UpdateVote(c) => Certificate::UpdateVote(UpdateVote(c)),
            certificate::Certificate::MintToken(c) => Certificate::MintToken(MintToken(c)),
            certificate::Certificate::EvmMapping(c) => Certificate::EvmMapping(EvmMapping(c)),
        }
    }
}

impl From<certificate::StakeDelegation> for StakeDelegation {
    fn from(delegation: certificate::StakeDelegation) -> StakeDelegation {
        StakeDelegation(delegation)
    }
}

impl From<certificate::OwnerStakeDelegation> for OwnerStakeDelegation {
    fn from(owner_stake_delegation: certificate::OwnerStakeDelegation) -> OwnerStakeDelegation {
        OwnerStakeDelegation(owner_stake_delegation)
    }
}

impl From<certificate::PoolRegistration> for PoolRegistration {
    fn from(registration: certificate::PoolRegistration) -> PoolRegistration {
        PoolRegistration(registration)
    }
}

impl From<certificate::PoolRetirement> for PoolRetirement {
    fn from(pool_retirement: certificate::PoolRetirement) -> PoolRetirement {
        PoolRetirement(pool_retirement)
    }
}

impl From<certificate::PoolUpdate> for PoolUpdate {
    fn from(pool_update: certificate::PoolUpdate) -> PoolUpdate {
        PoolUpdate(pool_update)
    }
}

impl From<certificate::VotePlan> for VotePlan {
    fn from(vote_plan: certificate::VotePlan) -> VotePlan {
        VotePlan(vote_plan)
    }
}

impl From<certificate::VoteCast> for VoteCast {
    fn from(vote_cast: certificate::VoteCast) -> VoteCast {
        VoteCast(vote_cast)
    }
}

impl From<certificate::UpdateProposal> for UpdateProposal {
    fn from(update_proposal: certificate::UpdateProposal) -> Self {
        UpdateProposal(update_proposal)
    }
}

impl From<certificate::UpdateVote> for UpdateVote {
    fn from(update_vote: certificate::UpdateVote) -> Self {
        UpdateVote(update_vote)
    }
}

impl From<certificate::EvmMapping> for EvmMapping {
    fn from(evm_mapping: certificate::EvmMapping) -> Self {
        EvmMapping(evm_mapping)
    }
}
