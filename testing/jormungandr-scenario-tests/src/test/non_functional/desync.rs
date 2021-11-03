use crate::{
    node::{LeadershipMode, PersistenceMode},
    scenario::repository::ScenarioResult,
    test::{non_functional::*, Result},
    Context,
};
use function_name::named;
use jormungandr_testing_utils::{testing::network::FaketimeConfig, wallet::Wallet};
use rand_chacha::ChaChaRng;

#[named]
pub fn bft_forks(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let name = function_name!();

    let n_transactions = 5;
    let transaction_amount = 1_000_000;
    let starting_funds = 100_000_000;

    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER_1,
            LEADER_2 -> LEADER_1,
            LEADER_3 -> LEADER_1,
        ]
        blockchain {
            consensus = Bft,
            number_of_slots_per_epoch = 60,
            slot_duration = 5,
            leaders = [ LEADER_1, LEADER_2, LEADER_3 ],
            initials = [
                "account" "alice" with starting_funds,
                "account" "bob" with starting_funds,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    let mut leader_1 = controller.spawn_node(
        LEADER_1,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader_1.wait_for_bootstrap()?;
    let mut leader_2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader_2.wait_for_bootstrap()?;
    let mut leader_3 = controller.spawn_node_custom(
        controller
            .new_spawn_params(LEADER_3)
            .leadership_mode(LeadershipMode::Leader)
            .persistence_mode(PersistenceMode::Persistent)
            .faketime(FaketimeConfig {
                offset: -2,
                drift: 0.0,
            }),
    )?;
    leader_3.wait_for_bootstrap()?;

    let mut alice = controller.wallet("alice")?;
    let bob = controller.wallet("bob")?;

    for i in 0..n_transactions {
        // Sooner or later this will fail because a transaction will settle
        // in the fork and the spending counter will not be correct anymore
        let mut alice_clone = alice.clone();
        controller.fragment_sender().send_transaction(
            &mut alice_clone,
            &bob,
            &leader_1,
            // done so each transaction is different even if the spending counter remains the same
            (transaction_amount + i).into(),
        )?;
        let state = leader_1.rest().account_state(&alice).unwrap();
        if let Wallet::Account(account) = &alice {
            let counter: u32 = account.internal_counter().into();
            if counter < state.counter() {
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

    leader_1.shutdown()?;
    leader_2.shutdown()?;
    leader_3.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed(name))
}
