use crate::testing::network::VotePlanKey;
use crate::testing::network::VotePlanSettings;
use crate::testing::network::{
    Blockchain as BlockchainTemplate, ExternalWalletTemplate, Node as NodeTemplate, NodeAlias,
    Random, Wallet, WalletAlias, WalletTemplate, WalletType,
};
use crate::wallet::PrivateVoteCommitteeDataManager;
use crate::{stake_pool::StakePool, testing::signed_stake_pool_cert, wallet::Wallet as WalletLib};
use assert_fs::fixture::ChildPath;
use assert_fs::fixture::PathChild;
use chain_crypto::Ed25519;
use chain_impl_mockchain::block::BlockDate;
use chain_impl_mockchain::testing::create_initial_vote_plan;
use chain_impl_mockchain::{
    certificate::VotePlan, chaintypes::ConsensusVersion, fee::LinearFee, vote::PayloadType,
};
use jormungandr_lib::crypto::account::Identifier;
use jormungandr_lib::interfaces::{try_initial_fragment_from_message, VotePlan as VotePLanLib};
use jormungandr_lib::{
    crypto::key::SigningKey,
    interfaces::{
        ActiveSlotCoefficient, Bft, Block0Configuration, BlockchainConfiguration, CommitteeIdDef,
        GenesisPraos, Initial, InitialUTxO, NodeConfig, NodeId, NodeSecret, TrustedPeer,
    },
};
use rand_core::{CryptoRng, RngCore};
use std::collections::{HashMap, HashSet};

/// contains all the data to start or interact with a node
#[derive(Debug, Clone)]
pub struct NodeSetting {
    /// for reference purpose only
    pub alias: NodeAlias,

    /// node secret, this will be passed to the node at start
    /// up of the node. It may contains the necessary crypto
    /// for the node to be a blockchain leader (BFT leader or
    /// stake pool)
    pub secret: NodeSecret,

    pub config: NodeConfig,

    pub topology_secret: SigningKey<Ed25519>,

    pub node_topology: NodeTemplate,
}

#[derive(Debug)]
pub struct Settings {
    pub nodes: HashMap<NodeAlias, NodeSetting>,

    pub wallets: HashMap<WalletAlias, Wallet>,

    pub block0: Block0Configuration,

    pub stake_pools: HashMap<NodeAlias, StakePool>,

    pub vote_plans: HashMap<VotePlanKey, VotePlanSettings>,
}

impl Settings {
    pub fn new<RNG>(
        nodes: HashMap<NodeAlias, NodeSetting>,
        blockchain: BlockchainTemplate,
        rng: &mut Random<RNG>,
    ) -> Self
    where
        RNG: RngCore + CryptoRng,
    {
        let mut settings = Settings {
            nodes,
            wallets: HashMap::new(),
            block0: Block0Configuration {
                blockchain_configuration: BlockchainConfiguration::new(
                    chain_addr::Discrimination::Test,
                    ConsensusVersion::Bft,
                    LinearFee::new(1, 2, 3),
                ),
                initial: Vec::new(),
            },
            stake_pools: HashMap::new(),
            vote_plans: HashMap::new(),
        };

        settings.populate_trusted_peers();
        settings.populate_block0_blockchain_initials(blockchain.wallets(), rng);
        settings.populate_block0_blockchain_configuration(&blockchain, rng);
        settings.populate_block0_blockchain_external(blockchain.external_wallets());
        settings.populate_block0_blockchain_vote_plans(
            blockchain.vote_plans(),
            blockchain.committees(),
            rng.rng_mut(),
        );
        settings
    }

    fn populate_block0_blockchain_external(
        &mut self,
        external_wallets: Vec<ExternalWalletTemplate>,
    ) {
        for template in external_wallets {
            let external_fragment = Initial::Fund(vec![InitialUTxO {
                address: template.address().parse().unwrap(),
                value: (*template.value()).into(),
            }]);
            self.block0.initial.push(external_fragment);
        }
    }

    fn populate_block0_blockchain_configuration<RNG>(
        &mut self,
        blockchain: &BlockchainTemplate,
        rng: &mut Random<RNG>,
    ) where
        RNG: RngCore + CryptoRng,
    {
        let mut blockchain_configuration = &mut self.block0.blockchain_configuration;

        // TODO blockchain_configuration.block0_date = ;
        blockchain_configuration.linear_fees = blockchain.linear_fee();
        blockchain_configuration.discrimination = blockchain.discrimination();
        blockchain_configuration.block0_date = blockchain.block0_date();
        blockchain_configuration.block0_consensus = *blockchain.consensus();
        blockchain_configuration.consensus_leader_ids = {
            if blockchain.has_external_consensus_leader_ids() {
                blockchain.external_consensus_leader_ids()
            } else {
                let mut leader_ids = Vec::new();
                for leader_alias in blockchain.leaders() {
                    let identifier = if let Some(node) = self.nodes.get_mut(leader_alias) {
                        if let Some(bft) = &node.secret.bft {
                            bft.signing_key.identifier()
                        } else {
                            let signing_key = SigningKey::generate(rng.rng_mut());
                            let identifier = signing_key.identifier();
                            node.secret.bft = Some(Bft { signing_key });
                            identifier
                        }
                    } else {
                        SigningKey::<Ed25519>::generate(rng.rng_mut()).identifier()
                    };
                    leader_ids.push(identifier.into());
                }
                leader_ids
            }
        };
        blockchain_configuration.committees = {
            let mut committees = Vec::new();
            for committee in blockchain.committees() {
                let wallet = self
                    .wallets
                    .get(&committee)
                    .unwrap_or_else(|| panic!("committee not defined {}", committee));
                committees.push(CommitteeIdDef::from(wallet.committee_id()));
            }
            committees.extend(&blockchain.external_committees());
            committees
        };
        blockchain_configuration.slots_per_epoch = *blockchain.slots_per_epoch();
        blockchain_configuration.slot_duration = *blockchain.slot_duration();
        blockchain_configuration.tx_max_expiry_epochs = blockchain.tx_max_expiry_epochs();
        blockchain_configuration.treasury = Some(1_000_000.into());
        blockchain_configuration.block_content_max_size = *blockchain.block_content_max_size();
        blockchain_configuration.kes_update_speed = *blockchain.kes_update_speed();
        blockchain_configuration.consensus_genesis_praos_active_slot_coeff =
            ActiveSlotCoefficient::MAXIMUM;
    }

    fn populate_block0_blockchain_initials<'a, RNG, I>(
        &'a mut self,
        wallet_templates: I,
        rng: &mut Random<RNG>,
    ) where
        RNG: RngCore + CryptoRng,
        I: Iterator<Item = &'a WalletTemplate>,
    {
        for wallet_template in wallet_templates {
            // TODO: check the wallet does not already exist ?
            let wallet = match wallet_template.wallet_type() {
                WalletType::UTxO => Wallet::generate_utxo(wallet_template.clone(), rng.rng_mut()),
                WalletType::Account => {
                    Wallet::generate_account(wallet_template.clone(), rng.rng_mut())
                }
            };

            let initial_address = wallet.address();

            // TODO add support for sharing fragment with multiple utxos
            let initial_fragment = Initial::Fund(vec![InitialUTxO {
                address: initial_address,
                value: (*wallet_template.value()).into(),
            }]);

            self.wallets
                .insert(wallet_template.alias().clone(), wallet.clone());
            self.block0.initial.push(initial_fragment);

            if let Some(delegation) = wallet_template.delegate() {
                use chain_impl_mockchain::certificate::PoolId as StakePoolId;

                // 1. retrieve the public data (we may need to create a stake pool
                //    registration here)
                let stake_pool_id: StakePoolId = if let Some(node) = self.nodes.get_mut(delegation)
                {
                    if let Some(genesis) = &node.secret.genesis {
                        genesis.node_id.into_digest_of()
                    } else {
                        // create and register the stake pool
                        let owner = WalletLib::new_account(&mut rand::rngs::OsRng);
                        let stake_pool = StakePool::new(&owner);
                        let node_id = stake_pool.id();
                        node.secret.genesis = Some(GenesisPraos {
                            sig_key: stake_pool.kes().signing_key(),
                            vrf_key: stake_pool.vrf().signing_key(),
                            node_id: {
                                let bytes: [u8; 32] = node_id.clone().into();
                                bytes.into()
                            },
                        });

                        self.block0.initial.push(Initial::Cert(
                            signed_stake_pool_cert(BlockDate::first().next_epoch(), &stake_pool)
                                .into(),
                        ));

                        self.stake_pools
                            .insert(delegation.clone(), stake_pool.clone());

                        node_id
                    }
                } else {
                    // delegating to a node that does not exist in the topology
                    // so generate valid stake pool registration and delegation
                    // to that node.
                    unimplemented!("delegating stake to a stake pool that is not a node is not supported (yet)")
                };

                // 2. create delegation certificate for the wallet stake key
                // and add it to the block0.initial array
                let delegation_certificate = wallet
                    .delegation_cert_for_block0(BlockDate::first().next_epoch(), stake_pool_id);

                self.block0.initial.push(delegation_certificate);
            }
        }
    }

    #[allow(deprecated)]
    fn populate_trusted_peers(&mut self) {
        //generate public id for all nodes treated as trusted peers
        let mut trusted_peers_aliases = HashSet::new();

        //gather aliases which are trusted peers
        for (_alias, node) in self.nodes.iter() {
            for trusted_peer in node.node_topology.trusted_peers.iter() {
                trusted_peers_aliases.insert(trusted_peer.clone());
            }
        }

        let nodes = self.nodes.clone();
        for (_alias, node) in self.nodes.iter_mut() {
            let mut trusted_peers = Vec::new();

            for trusted_peer in node.node_topology.trusted_peers.iter() {
                let trusted_peer = nodes.get(trusted_peer).unwrap();
                let id = NodeId::from(
                    <chain_crypto::SecretKey<chain_crypto::Ed25519>>::generate(rand::thread_rng())
                        .to_public(),
                );
                trusted_peers.push(TrustedPeer {
                    address: trusted_peer.config.p2p.public_address.clone(),
                    id: Some(id),
                })
            }

            node.config.skip_bootstrap = Some(trusted_peers.is_empty());
            node.config.bootstrap_from_trusted_peers = Some(!trusted_peers.is_empty());
            node.config.p2p.trusted_peers = trusted_peers;
        }
    }

    fn populate_block0_blockchain_vote_plans<RNG>(
        &mut self,
        mut vote_plans: HashMap<VotePlanKey, VotePLanLib>,
        committees_aliases: Vec<WalletAlias>,
        rng: &mut RNG,
    ) where
        RNG: RngCore + CryptoRng,
    {
        let mut vote_plans_fragments = Vec::new();

        for (vote_plan_key, vote_plan) in vote_plans.iter_mut() {
            let owner = self
                .wallets
                .get(&vote_plan_key.owner_alias)
                .unwrap_or_else(|| {
                    panic!(
                        "Owner {} of {} is unknown wallet ",
                        vote_plan_key.owner_alias, vote_plan_key.alias
                    )
                });

            // workaround beacuse vote_plan_def does not expose payload_type
            let tmp_vote_plan: VotePlan = vote_plan.clone().into();

            let vote_plan_settings: VotePlanSettings = match tmp_vote_plan.payload_type() {
                PayloadType::Public => VotePlanSettings::from_public_vote_plan(vote_plan.clone()),
                PayloadType::Private => {
                    let mut wallets = self.wallets.clone();
                    let committees: Vec<(WalletAlias, Identifier)> = committees_aliases
                        .iter()
                        .map(|x| {
                            let wallet = wallets.get_mut(x).unwrap();
                            (x.clone(), wallet.identifier().into())
                        })
                        .collect();
                    let threshold = committees.len();
                    let keys = PrivateVoteCommitteeDataManager::new(rng, committees, threshold);

                    vote_plan
                        .committee_member_public_keys
                        .extend(keys.member_public_keys());
                    VotePlanSettings::Private {
                        keys,
                        vote_plan: vote_plan.clone(),
                    }
                }
            };

            vote_plans_fragments.push(create_initial_vote_plan(
                &vote_plan_settings.vote_plan().into(),
                &[owner.clone().into()],
            ));

            self.vote_plans
                .insert(vote_plan_key.clone(), vote_plan_settings);
        }
        self.block0.initial.extend(
            vote_plans_fragments
                .iter()
                .map(|message| try_initial_fragment_from_message(message).unwrap()),
        )
    }

    pub fn dump_private_vote_keys(&self, directory: ChildPath) {
        for (vote_plan_alias, data) in self.vote_plans.iter() {
            if let VotePlanSettings::Private { keys, vote_plan: _ } = data {
                let (_, vote_plan) = self
                    .vote_plans
                    .iter()
                    .find(|(alias, _)| *alias == vote_plan_alias)
                    .unwrap();

                let vote_plan_dir = directory.child(format!("{}_committees", vote_plan.to_id()));
                keys.write_to(vote_plan_dir).unwrap();
            }
        }
    }
}
