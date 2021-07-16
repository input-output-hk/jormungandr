#![allow(dead_code)]

pub mod connections;
pub mod stats;

pub use connections::max_connections;
pub use stats::p2p_stats_test;

use crate::{
    node::NodeController,
    test::{utils, Result},
};

use jormungandr_lib::interfaces::PeerRecord;

pub fn assert_connected_cnt(
    node: &NodeController,
    peer_connected_cnt: usize,
    info: &str,
) -> Result<()> {
    let stats = node.stats()?.stats.expect("empty stats");
    Ok(utils::assert_equals(
        &peer_connected_cnt,
        &stats.peer_connected_cnt.clone(),
        &format!("{}: peer_connected_cnt, Node {}", info, node.alias()),
    )?)
}

pub fn assert_node_stats(
    node: &NodeController,
    peer_available_cnt: usize,
    peer_quarantined_cnt: usize,
    peer_total_cnt: usize,
    info: &str,
) -> Result<()> {
    node.log_stats();
    let stats = node.stats()?.stats.expect("empty stats");
    utils::assert_equals(
        &peer_available_cnt,
        &stats.peer_available_cnt.clone(),
        &format!("{}: peer_available_cnt, Node {}", info, node.alias()),
    )?;

    utils::assert_equals(
        &peer_quarantined_cnt,
        &stats.peer_quarantined_cnt,
        &format!("{}: peer_quarantined_cnt, Node {}", info, node.alias()),
    )?;
    utils::assert_equals(
        &peer_total_cnt,
        &stats.peer_total_cnt,
        &format!("{}: peer_total_cnt, Node {}", info, node.alias()),
    )?;

    Ok(())
}

pub fn assert_are_in_network_view(
    node: &NodeController,
    peers: Vec<&NodeController>,
    info: &str,
) -> Result<()> {
    let network_view = node.p2p_view()?;
    for peer in peers {
        utils::assert(
            network_view
                .iter()
                .any(|address| *address == peer.address().to_string()),
            &format!(
                "{}: Peer {} is not present in network view list",
                info,
                peer.alias()
            ),
        )?;
    }
    Ok(())
}

pub fn assert_are_not_in_network_view(
    node: &NodeController,
    peers: Vec<&NodeController>,
    info: &str,
) -> Result<()> {
    let network_view = node.network_stats()?;
    for peer in peers {
        utils::assert(
            network_view
                .iter()
                .any(|info| info.addr == Some(peer.address())),
            &format!(
                "{}: Peer {} is present in network view list, while it should not",
                info,
                peer.alias()
            ),
        )?;
    }
    Ok(())
}

pub fn assert_are_in_network_stats(
    node: &NodeController,
    peers: Vec<&NodeController>,
    info: &str,
) -> Result<()> {
    let network_stats = node.network_stats()?;
    for peer in peers {
        utils::assert(
            network_stats.iter().any(|x| x.addr == Some(peer.address())),
            &format!(
                "{}: Peer {} is not present in network_stats list",
                info,
                peer.alias()
            ),
        )?;
    }
    Ok(())
}

pub fn assert_are_not_in_network_stats(
    node: &NodeController,
    peers: Vec<&NodeController>,
    info: &str,
) -> Result<()> {
    let network_stats = node.network_stats()?;
    for peer in peers {
        utils::assert(
            !network_stats.iter().any(|x| x.addr == Some(peer.address())),
            &format!(
                "{}: Peer {} is present in network_stats list, while it should not",
                info,
                peer.alias()
            ),
        )?;
    }
    Ok(())
}

pub fn assert_are_available(
    node: &NodeController,
    peers: Vec<&NodeController>,
    info: &str,
) -> Result<()> {
    let available_list = node.p2p_available()?;
    assert_record_is_present(available_list, peers, "available", info)
}

pub fn assert_are_not_available(
    node: &NodeController,
    peers: Vec<&NodeController>,
    info: &str,
) -> Result<()> {
    let available_list = node.p2p_available()?;
    assert_record_is_present(available_list, peers, "available", info)
}

pub fn assert_empty_quarantine(node: &NodeController, info: &str) -> Result<()> {
    let quarantine = node.p2p_quarantined()?;
    Ok(utils::assert_equals(
        &vec![],
        &quarantine,
        &format!(
            "{}: Peer {} has got non empty quarantine list",
            info,
            node.alias()
        ),
    )?)
}

pub fn assert_are_in_quarantine(
    node: &NodeController,
    peers: Vec<&NodeController>,
    info: &str,
) -> Result<()> {
    let available_list = node.p2p_quarantined()?;
    assert_record_is_present(available_list, peers, "quarantine", info)
}

pub fn assert_record_is_present(
    peer_list: Vec<PeerRecord>,
    peers: Vec<&NodeController>,
    list_name: &str,
    info: &str,
) -> Result<()> {
    for peer in peers {
        utils::assert(
            peer_list
                .iter()
                .any(|x| x.address == peer.address().to_string()),
            &format!(
                "{}: Peer {} is not present in {} list",
                info,
                peer.alias(),
                list_name
            ),
        )?;
    }
    Ok(())
}

pub fn assert_record_is_not_present(
    peer_list: Vec<PeerRecord>,
    peers: Vec<&NodeController>,
    list_name: &str,
) -> Result<()> {
    for peer in peers {
        utils::assert(
            !peer_list
                .iter()
                .any(|x| x.address == peer.address().to_string()),
            &format!(
                "Peer {} is present in {} list, while should not",
                peer.alias(),
                list_name
            ),
        )?;
    }
    Ok(())
}
