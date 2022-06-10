pub mod connections;
pub mod quarantine;
pub mod stats;

pub use connections::max_connections;
use jormungandr_automation::jormungandr::JormungandrProcess;
use jormungandr_lib::interfaces::PeerRecord;
pub use stats::p2p_stats_test;

pub fn assert_connected_cnt(node: &JormungandrProcess, peer_connected_cnt: usize, info: &str) {
    let stats = node.rest().stats().unwrap().stats.expect("empty stats");
    assert_eq!(
        &peer_connected_cnt,
        &stats.peer_connected_cnt.clone(),
        "{}: peer_connected_cnt, Node {}",
        info,
        node.alias()
    );
}

pub fn assert_node_stats(
    node: &JormungandrProcess,
    peer_available_cnt: usize,
    peer_quarantined_cnt: usize,
    peer_total_cnt: usize,
    info: &str,
) {
    node.log_stats();
    let stats = node.rest().stats().unwrap().stats.expect("empty stats");
    assert_eq!(
        &peer_available_cnt,
        &stats.peer_available_cnt.clone(),
        "{}: peer_available_cnt, Node {}",
        info,
        node.alias()
    );

    assert_eq!(
        &peer_quarantined_cnt,
        &stats.peer_quarantined_cnt,
        "{}: peer_quarantined_cnt, Node {}",
        info,
        node.alias()
    );
    assert_eq!(
        &peer_total_cnt,
        &stats.peer_total_cnt,
        "{}: peer_total_cnt, Node {}",
        info,
        node.alias()
    );
}

pub fn assert_are_in_network_view(
    node: &JormungandrProcess,
    peers: Vec<&JormungandrProcess>,
    info: &str,
) {
    let network_view = node.rest().p2p_view().unwrap();
    for peer in peers {
        assert!(
            network_view
                .iter()
                .any(|address| *address == peer.address().to_string()),
            "{}: Peer {} is not present in network view list",
            info,
            peer.alias()
        );
    }
}

pub fn assert_are_not_in_network_view(
    node: &JormungandrProcess,
    peers: Vec<&JormungandrProcess>,
    info: &str,
) {
    let network_view = node.rest().network_stats().unwrap();
    for peer in peers {
        assert!(
            network_view
                .iter()
                .any(|info| info.addr == Some(peer.address())),
            "{}: Peer {} is present in network view list, while it should not",
            info,
            peer.alias()
        );
    }
}

pub fn assert_are_in_network_stats(
    node: &JormungandrProcess,
    peers: Vec<&JormungandrProcess>,
    info: &str,
) {
    let network_stats = node.rest().network_stats().unwrap();
    for peer in peers {
        assert!(
            network_stats.iter().any(|x| x.addr == Some(peer.address())),
            "{}: Peer {} is not present in network_stats list",
            info,
            peer.alias()
        );
    }
}

pub fn assert_are_not_in_network_stats(
    node: &JormungandrProcess,
    peers: Vec<&JormungandrProcess>,
    info: &str,
) {
    let network_stats = node.rest().network_stats().unwrap();
    for peer in peers {
        assert!(
            !network_stats.iter().any(|x| x.addr == Some(peer.address())),
            "{}: Peer {} is present in network_stats list, while it should not",
            info,
            peer.alias()
        );
    }
}

pub fn assert_are_available(
    node: &JormungandrProcess,
    peers: Vec<&JormungandrProcess>,
    info: &str,
) {
    let available_list = node.rest().p2p_available().unwrap();
    assert_record_is_present(available_list, peers, "available", info)
}

pub fn assert_are_not_available(
    node: &JormungandrProcess,
    peers: Vec<&JormungandrProcess>,
    info: &str,
) {
    let available_list = node.rest().p2p_available().unwrap();
    assert_record_is_present(available_list, peers, "available", info)
}

pub fn assert_empty_quarantine(node: &JormungandrProcess, info: &str) {
    let quarantine = node.rest().p2p_quarantined().unwrap();
    assert_eq!(
        quarantine,
        vec![],
        "{}: Peer {} has got non empty quarantine list",
        info,
        node.alias()
    )
}

pub fn assert_are_in_quarantine(
    node: &JormungandrProcess,
    peers: Vec<&JormungandrProcess>,
    info: &str,
) {
    let available_list = node.rest().p2p_quarantined().unwrap();
    assert_record_is_present(available_list, peers, "quarantine", info)
}

pub fn assert_record_is_present(
    peer_list: Vec<PeerRecord>,
    peers: Vec<&JormungandrProcess>,
    list_name: &str,
    info: &str,
) {
    for peer in peers {
        assert!(
            peer_list
                .iter()
                .any(|x| x.address == peer.address().to_string()),
            "{}: Peer {} is not present in {} list",
            info,
            peer.alias(),
            list_name
        );
    }
}

pub fn assert_record_is_not_present(
    peer_list: Vec<PeerRecord>,
    peers: Vec<&JormungandrProcess>,
    list_name: &str,
) {
    for peer in peers {
        assert!(
            !peer_list
                .iter()
                .any(|x| x.address == peer.address().to_string()),
            "Peer {} is present in {} list, while should not",
            peer.alias(),
            list_name
        );
    }
}
