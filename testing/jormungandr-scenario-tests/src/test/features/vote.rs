use crate::{
    node::{LeadershipMode, PersistenceMode},
    test::{utils, Result},
    Context, ScenarioResult,
};
use function_name::named;
use jormungandr_lib::interfaces::Explorer;
use jormungandr_testing_utils::testing::network_builder::SpawnParams;
use jormungandr_testing_utils::testing::node::time;
use rand_chacha::ChaChaRng;
use vit_servicing_station_tests::common::data::ValidVotePlanParameters;
const LEADER_1: &str = "Leader1";
const LEADER_2: &str = "Leader2";
const LEADER_3: &str = "Leader3";
const LEADER_4: &str = "Leader4";
const WALLET_NODE: &str = "Wallet_Node";

const DAVID_ADDRESS: &str = "DdzFFzCqrhsktawSMCWJJy3Dpp9BCjYPVecgsMb5U2G7d1ErUUmwSZvfSY3Yjn5njNadfwvebpVNS5cD4acEKSQih2sR76wx2kF4oLXT";
const DAVID_MNEMONICS: &str =
    "tired owner misery large dream glad upset welcome shuffle eagle pulp time";

const EDGAR_ADDRESS: &str = "DdzFFzCqrhsf2sWcZLzXhyLoLZcmw3Zf3UcJ2ozG1EKTwQ6wBY1wMG1tkXtPvEgvE5PKUFmoyzkP8BL4BwLmXuehjRHJtnPj73E5RPMx";
const EDGAR_MNEMONICS: &str =
    "edge club wrap where juice nephew whip entry cover bullet cause jeans";

const FILIP_MNEMONICS: &str =
    "neck bulb teach illegal soul cry monitor claw amount boring provide village rival draft stone";
const FILIP_ADDRESS: &str = "Ae2tdPwUPEZ8og5u4WF5rmSyme5Gvp8RYiLM2u7Vm8CyDQzLN3VYTN895Wk";

#[allow(dead_code)]
pub enum Vote {
    BLANK = 0,
    YES = 1,
    NO = 2,
}

#[named]
pub fn vote_e2e_flow(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER_1,
            LEADER_2 -> LEADER_1,
            LEADER_3 -> LEADER_1,
            LEADER_4 -> LEADER_1,
            WALLET_NODE -> LEADER_1,LEADER_2,LEADER_3,LEADER_4
        ]
        blockchain {
            consensus = Bft,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1, LEADER_2, LEADER_3, LEADER_4 ],
            initials = [
                "account" "Alice" with 500_000_000,
            ],
            committees = [ "Alice" ],
            legacy = [
                "David" address DAVID_ADDRESS mnemonics DAVID_MNEMONICS with 500_000_000,
                "Edgar" address EDGAR_ADDRESS mnemonics EDGAR_MNEMONICS with 500_000_000,
                "Filip" address FILIP_ADDRESS mnemonics FILIP_MNEMONICS with 500_000_000,
            ],
            vote_plans = [
                "fund1" from "Alice" through epochs 0->1->2 as "public" contains proposals = [
                    proposal adds 100 to "rewards" with 3 vote options,
                ]
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    // bootstrap network
    let leader_1 = controller.spawn_node_custom(
        SpawnParams::new(LEADER_1)
            .leader()
            .persistence_mode(PersistenceMode::Persistent)
            .explorer(Explorer { enabled: true }),
    )?;
    leader_1.wait_for_bootstrap()?;
    controller.monitor_nodes();

    //start bft node 2
    let leader_2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader_2.wait_for_bootstrap()?;

    //start bft node 3
    let leader_3 = controller.spawn_node(
        LEADER_3,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader_3.wait_for_bootstrap()?;

    //start bft node 4
    let leader_4 = controller.spawn_node(
        LEADER_4,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader_4.wait_for_bootstrap()?;

    // start passive node
    let wallet_node = controller.spawn_node_custom(
        SpawnParams::new(WALLET_NODE)
            .passive()
            .persistence_mode(PersistenceMode::Persistent)
            .explorer(Explorer { enabled: true }),
    )?;
    wallet_node.wait_for_bootstrap()?;
    let fund1_vote_plan = controller.vote_plan("fund1")?;

    // start proxy and vit station
    let vit_station =
        controller.spawn_vit_station(ValidVotePlanParameters::new(fund1_vote_plan))?;
    let wallet_proxy = controller.spawn_wallet_proxy(WALLET_NODE)?;

    // start mainnet walets
    let mut david = controller.iapyx_wallet(DAVID_MNEMONICS, &wallet_proxy)?;
    david.retrieve_funds()?;
    david.convert_and_send()?;

    let fund1_vote_plan = controller.vote_plan("fund1")?;

    // start voting
    david.vote_for(fund1_vote_plan.id(), 0, Vote::YES as u8)?;

    let mut edgar = controller.iapyx_wallet(EDGAR_MNEMONICS, &wallet_proxy)?;
    edgar.retrieve_funds()?;
    edgar.convert_and_send()?;

    edgar.vote_for(fund1_vote_plan.id(), 0, Vote::YES as u8)?;

    let mut filip = controller.iapyx_wallet(FILIP_MNEMONICS, &wallet_proxy)?;
    filip.retrieve_funds()?;
    filip.convert_and_send()?;

    filip.vote_for(fund1_vote_plan.id(), 0, Vote::NO as u8)?;

    time::wait_for_epoch(1, leader_1.explorer());

    //tally the vote and observe changes
    let rewards_before = leader_1
        .explorer()
        .status()
        .unwrap()
        .data
        .unwrap()
        .status
        .latest_block
        .treasury
        .unwrap()
        .rewards
        .parse::<u64>()
        .unwrap();

    let mut alice = controller.wallet("Alice")?;
    controller.fragment_sender().send_public_vote_tally(
        &mut alice,
        &fund1_vote_plan.into(),
        &wallet_node,
    )?;

    time::wait_for_epoch(2, leader_1.explorer());

    let rewards_after = leader_1
        .explorer()
        .status()
        .unwrap()
        .data
        .unwrap()
        .status
        .latest_block
        .treasury
        .unwrap()
        .rewards
        .parse::<u64>()
        .unwrap();

    utils::assert_equals(
        &rewards_before,
        &(rewards_after - 100),
        &format!(
            "{} <> {} rewards were not increased",
            rewards_before, rewards_after
        ),
    )?;

    wallet_node.shutdown()?;
    vit_station.shutdown();
    wallet_proxy.shutdown();
    leader_4.shutdown()?;
    leader_3.shutdown()?;
    leader_2.shutdown()?;
    leader_1.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed(name))
}
