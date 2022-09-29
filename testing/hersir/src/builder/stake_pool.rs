use crate::builder::{NodeSetting, Wallet};
use chain_impl_mockchain::block::BlockDate;
use jormungandr_automation::jormungandr::NodeAlias;
use jormungandr_lib::interfaces::{GenesisPraos, Initial};
use std::collections::HashMap;
use thor::{signed_stake_pool_cert, StakePool};

pub fn generate(
    wallets: &Vec<Wallet>,
    nodes: &mut HashMap<NodeAlias, NodeSetting>,
) -> Result<(Vec<Initial>, HashMap<NodeAlias, StakePool>), super::settings::Error> {
    let mut initials = Vec::new();
    let mut stake_pools = HashMap::new();

    for wallet in wallets {
        if let Some(delegation) = wallet.template().delegate() {
            use chain_impl_mockchain::certificate::PoolId as StakePoolId;

            // 1. retrieve the public data (we may need to create a stake pool
            //    registration here)
            let stake_pool_id: StakePoolId = if let Some(node) = nodes.get_mut(delegation) {
                if let Some(genesis) = &node.secret.genesis {
                    genesis.node_id.into_digest_of()
                } else {
                    // create and register the stake pool
                    let owner = thor::Wallet::new_account(
                        &mut rand::rngs::OsRng,
                        wallet.template().discrimination(),
                    );
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

                    initials.push(Initial::Cert(
                        signed_stake_pool_cert(BlockDate::first().next_epoch(), &stake_pool).into(),
                    ));

                    stake_pools.insert(delegation.clone(), stake_pool.clone());

                    node_id
                }
            } else {
                // delegating to a node that does not exist in the topology
                // so generate valid stake pool registration and delegation
                // to that node.
                unimplemented!(
                    "delegating stake to a stake pool that is not a node is not supported (yet)"
                )
            };

            // 2. create delegation certificate for the wallet stake key
            // and add it to the block0.initial array
            let delegation_certificate = wallet
                .delegation_cert_for_block0(BlockDate::first().next_epoch(), stake_pool_id)?;

            initials.push(delegation_certificate);
        }
    }
    Ok((initials, stake_pools))
}
