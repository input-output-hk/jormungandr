use crate::{
    node::NodeController,
    node::{LeadershipMode, PersistenceMode},
    test::{utils, Result},
    Context, ScenarioResult,
};
use jormungandr_lib::interfaces::EnclaveLeaderId;
use rand_chacha::ChaChaRng;
const LEADER1: &str = "LEADER1";
const LEADER2: &str = "LEADER2";
const LEADER3: &str = "LEADER3";
const LEADER4: &str = "LEADER4";

pub fn p2p_stats_test(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "p2p stats",
        &mut context,
        topology [
            LEADER1,
            LEADER2 -> LEADER1,
            LEADER3 -> LEADER1,
            LEADER4 -> LEADER2 -> LEADER3,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 2,
            leaders = [ LEADER1 ],
            initials = [
                account "delegated1" with  2_000_000_000 delegates to LEADER1,
                account "delegated2" with  2_000_000_000 delegates to LEADER2,
                account "delegated3" with  2_000_000_000 delegates to LEADER3,
                account "delegated4" with  2_000_000_000 delegates to LEADER4,
            ],
        }  
    };

    let mut controller = scenario_settings.build(context)?;

    let leader1 =
        controller.spawn_node(LEADER1, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    leader1.wait_for_bootstrap()?;

    let leader1_node_id = leader1.stats()?.stats.expect("empty stats").node_id.clone();
    assert_node_stats(&leader1,0,0,0,0,0)?;


    utils::assert_equals(&vec![],&leader1.network_stats()?,"network_stats")?;
    utils::assert_equals(&vec![],&leader1.p2p_quarantined()?,"p2p_quarantined")?;
    utils::assert_equals(&vec![],&leader1.p2p_non_public()?,"p2p_non_public")?;
    utils::assert_equals(&vec![],&leader1.p2p_available()?,"p2p_available")?;
    utils::assert_equals(&vec![],&leader1.p2p_view()?,"p2p_view")?;


    let leader2 =
        controller.spawn_node(LEADER2, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    leader2.wait_for_bootstrap()?;
    
    utils::wait(90);
    assert_node_stats(&leader1,1,1,0,1,0)?;
    assert_node_stats(&leader2,1,1,0,1,0)?;
   /* 
    let leader3 =
        controller.spawn_node(LEADER3, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    leader3.wait_for_bootstrap()?;
    */
 /*   let leader4 =
        controller.spawn_node(LEADER4, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    leader4.wait_for_bootstrap()?;
    
    utils::wait(90);

    
    println!("{:?}",leader1.stats()?.stats.expect("empty stats"));
    println!("{:?}",leader1.network_stats());
    println!("{:?}",leader1.p2p_quarantined());
    println!("{:?}",leader1.p2p_non_public());
    println!("{:?}",leader1.p2p_available());
    println!("{:?}",leader1.p2p_view());


    println!("{:?}",leader2.stats()?.stats.expect("empty stats"));
    println!("{:?}",leader2.network_stats());
    println!("{:?}",leader2.p2p_quarantined());
    println!("{:?}",leader2.p2p_non_public());
    println!("{:?}",leader2.p2p_available());
    println!("{:?}",leader2.p2p_view());

    println!("{:?}",leader3.stats()?.stats.expect("empty stats"));
    println!("{:?}",leader3.network_stats());
    println!("{:?}",leader3.p2p_quarantined());
    println!("{:?}",leader3.p2p_non_public());
    println!("{:?}",leader3.p2p_available());
    println!("{:?}",leader3.p2p_view());
*/
  //  assert_node_stats(&leader1,3,4,0,3,0)?;
   // assert_node_stats(&leader2,3,4,0,3,0)?;
   // assert_node_stats(&leader3,3,4,0,3,0)?;
 //   assert_node_stats(&leader4,3,4,0,3,0)?;

/*
    leader2.shutdown()?;

    utils::wait(60);

    println!("{:?}",leader1.network_stats());
    println!("{:?}",leader1.p2p_quarantined());
    println!("{:?}",leader1.p2p_non_public());
    println!("{:?}",leader1.p2p_available());
    println!("{:?}",leader1.p2p_view());

    assert_node_stats(&leader1,2,3,1,4,1)?;


    let leader2 =
        controller.spawn_node(LEADER2, LeadershipMode::Leader, PersistenceMode::Persistent)?;
    leader2.wait_for_bootstrap()?;
    

    utils::wait(40);

    assert_node_stats(&leader1,3,4,0,3,0);
    assert_node_stats(&leader2,3,4,0,3,0);
    assert_node_stats(&leader3,3,4,0,3,0);
    assert_node_stats(&leader4,3,4,0,3,0);

*/
    leader1.shutdown()?;
    leader2.shutdown()?;
  //  leader3.shutdown()?;
  //  leader4.shutdown()?;
    controller.finalize();
    Ok(ScenarioResult::passed())
}

fn assert_node_stats(node: &NodeController,
    peer_available_cnt: usize, 
    peer_connected_cnt: usize, 
    peer_quarantined_cnt: usize, 
    peer_total_cnt: usize, 
    peer_unreachable_cnt: usize
) -> Result<()> {
    
    node.log_stats();    
    let stats = node.stats()?.stats.expect("empty stats");
    utils::assert_equals(&peer_available_cnt,&stats.peer_available_cnt.clone(),&format!("peer_available_cnt, Node {}",node.alias()))?;
    utils::assert_equals(&peer_connected_cnt,&stats.peer_connected_cnt,&format!("peer_connected_cnt, Node {}",node.alias()))?;
    utils::assert_equals(&peer_quarantined_cnt,&stats.peer_quarantined_cnt,&format!("peer_quarantined_cnt, Node {}",node.alias()))?;
    utils::assert_equals(&peer_total_cnt,&stats.peer_total_cnt,&format!("peer_total_cnt, Node {}",node.alias()))?;
    utils::assert_equals(&peer_unreachable_cnt,&stats.peer_unreachable_cnt,&format!("peer_unreachable_cnt, Node {}",node.alias()))?;

    
    Ok(())
}
