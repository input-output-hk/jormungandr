use crate::non_functional::network::*;
use chain_impl_mockchain::chaintypes::ConsensusVersion;
use hersir::builder::blockchain::BlockchainBuilder;
use hersir::builder::wallet::template::builder::WalletTemplateBuilder;
use hersir::builder::NetworkBuilder;
use hersir::builder::Node;
use hersir::builder::SpawnParams;
use hersir::builder::Topology;
use jormungandr_testing_utils::testing::jormungandr::FaketimeConfig;
use jormungandr_testing_utils::testing::FragmentSender;
use jormungandr_testing_utils::wallet::Wallet;

#[test]
pub fn bft_forks() {
    let n_transactions = 5;
    let transaction_amount = 1_000_000;
    let starting_funds = 100_000_000;

    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_3))
                .with_node(
                    Node::new(LEADER_1)
                        .with_trusted_peer(LEADER_3)
                        .with_trusted_peer(LEADER_4),
                )
                .with_node(Node::new(LEADER_2).with_trusted_peer(LEADER_1))
                .with_node(
                    Node::new(LEADER_4)
                        .with_trusted_peer(LEADER_2)
                        .with_trusted_peer(LEADER_3),
                ),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_500_000_000)
                .build(),
        )
        .wallet_template(WalletTemplateBuilder::new(BOB).with(2_000_000_000).build())
        .blockchain_config(
            BlockchainBuilder::default()
                .consensus(ConsensusVersion::Bft)
                .slots_per_epoch(60)
                .slot_duration(5)
                .build(),
        )
        .build()
        .unwrap();

    let leader_1 = controller.spawn(SpawnParams::new(LEADER_1)).unwrap();

    let _leader_2 = controller.spawn(SpawnParams::new(LEADER_2)).unwrap();
    let _leader_3 = controller
        .spawn(SpawnParams::new(LEADER_3).faketime(FaketimeConfig {
            offset: -2,
            drift: 0.0,
        }))
        .unwrap();

    let mut alice = controller.wallet(ALICE).unwrap();
    let bob = controller.wallet(BOB).unwrap();

    for i in 0..n_transactions {
        // Sooner or later this will fail because a transaction will settle
        // in the fork and the spending counter will not be correct anymore
        let mut alice_clone = alice.clone();
        FragmentSender::from(&controller.settings().block0)
            .send_transaction(
                &mut alice_clone,
                &bob,
                &leader_1,
                // done so each transaction is different even if the spending counter remains the same
                (transaction_amount + i).into(),
            )
            .unwrap();
        let state = leader_1.rest().account_state(&alice).unwrap();
        // The fragment sender currently only uses the counter in lane 0
        let updated_counter = state.counters()[0];
        if let Wallet::Account(account) = &alice {
            let counter: u32 = account.internal_counter().into();
            if counter < updated_counter {
                alice.confirm_transaction();
            }
        }
        // Spans at least one slot for every leader
        std::thread::sleep(std::time::Duration::from_secs(5));
    }

    let account_value: u64 = (*leader_1.rest().account_state(&alice).unwrap().value()).into();
    assert!(
        account_value < starting_funds - transaction_amount * n_transactions,
        "found {}",
        account_value
    );
}
