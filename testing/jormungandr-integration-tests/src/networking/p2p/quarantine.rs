use crate::networking::{
    p2p::{assert_are_in_quarantine, assert_empty_quarantine, assert_node_stats},
    utils,
};
use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{Blockchain, SpawnParams, WalletTemplateBuilder},
};
use jormungandr_lib::{interfaces::Policy, time::Duration};

const CLIENT: &str = "CLIENT";
const CLIENT_2: &str = "CLIENT_2";
const SERVER: &str = "SERVER";

const ALICE: &str = "ALICE";
const BOB: &str = "BOB";

#[test]
pub fn node_whitelist_itself() {
    let mut network_controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(SERVER))
                .with_node(Node::new(CLIENT).with_trusted_peer(SERVER)),
        )
        .blockchain_config(Blockchain::default().with_leader(SERVER))
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(1_000_000)
                .delegated_to(CLIENT)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(1_000_000)
                .delegated_to(SERVER)
                .build(),
        )
        .build()
        .unwrap();

    let _server = network_controller.spawn(SpawnParams::new(SERVER).in_memory());

    let client_public_address = network_controller
        .node_config(CLIENT)
        .unwrap()
        .p2p
        .public_address;
    let policy = Policy {
        quarantine_duration: Some(Duration::new(1, 0)),
        quarantine_whitelist: Some(vec![client_public_address]),
    };

    let client = network_controller
        .spawn(SpawnParams::new(CLIENT).policy(policy))
        .unwrap();
    client.assert_no_errors_in_log();
}

#[test]
pub fn node_does_not_quarantine_whitelisted_node() {
    let mut network_controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(SERVER))
                .with_node(Node::new(CLIENT).with_trusted_peer(SERVER))
                .with_node(Node::new(CLIENT_2).with_trusted_peer(SERVER)),
        )
        .blockchain_config(Blockchain::default().with_leader(SERVER))
        .build()
        .unwrap();

    let fake_addr = "/ip4/127.0.0.1/tcp/80";
    let policy = Policy {
        quarantine_duration: Some(Duration::new(30, 0)),
        quarantine_whitelist: Some(vec![fake_addr.parse().unwrap()]),
    };

    let server = network_controller.spawn(SpawnParams::new(SERVER)).unwrap();

    let _client = network_controller
        .spawn(
            SpawnParams::new(CLIENT)
                // The client broadcast a different ip address from the one it's actually
                // listening to, so that client_2 will fail connection
                .public_address("/ip4/127.0.0.1/tcp/80".parse().unwrap())
                .listen_address(Some(
                    network_controller
                        .node_config(CLIENT)
                        .unwrap()
                        .p2p
                        .get_listen_addr()
                        .unwrap(),
                )),
        )
        .unwrap();

    assert_node_stats(&server, 1, 0, 1, "before starting client2");
    assert_empty_quarantine(&server, "before starting client2");

    let client2 = network_controller
        .spawn(SpawnParams::new(CLIENT_2).policy(policy).in_memory())
        .unwrap();

    utils::wait(20);

    assert_node_stats(&client2, 2, 0, 2, "after starting client2");
    assert_empty_quarantine(&client2, "after starting client2");
}

#[test]
pub fn node_put_in_quarantine_nodes_which_are_not_whitelisted() {
    let mut network_controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(SERVER))
                .with_node(Node::new(CLIENT).with_trusted_peer(SERVER))
                .with_node(Node::new(CLIENT_2).with_trusted_peer(SERVER)),
        )
        .blockchain_config(Blockchain::default().with_leader(SERVER))
        .build()
        .unwrap();

    let server = network_controller
        .spawn(SpawnParams::new(SERVER).in_memory())
        .unwrap();

    let client = network_controller
        .spawn(
            SpawnParams::new(CLIENT)
                // The client broadcast a different ip address from the one it's actually
                // listening to, so that client_2 will fail connection and put it in quarantine
                .public_address("/ip4/127.0.0.1/tcp/80".parse().unwrap())
                .listen_address(Some(
                    network_controller
                        .node_config(CLIENT)
                        .unwrap()
                        .p2p
                        .get_listen_addr()
                        .unwrap(),
                )),
        )
        .unwrap();

    assert_node_stats(&server, 1, 0, 1, "before starting client2");
    assert_empty_quarantine(&server, "before starting client2");

    utils::wait(20);

    let client2 = network_controller
        .spawn(
            SpawnParams::new(CLIENT_2)
                .in_memory()
                // The client broadcast a different ip address from the one it's actually
                // listening to, so that client will fail connection and put it in quarantine
                .public_address("/ip4/127.0.0.1/tcp/810".parse().unwrap())
                .listen_address(Some(
                    network_controller
                        .node_config(CLIENT_2)
                        .unwrap()
                        .p2p
                        .get_listen_addr()
                        .unwrap(),
                )),
        )
        .unwrap();

    utils::wait(20);

    assert_node_stats(&client2, 1, 1, 2, "after starting client2");
    assert_are_in_quarantine(&client2, vec![&client], "after starting client2");
    assert_node_stats(&client, 1, 1, 2, "after starting client2");
    assert_are_in_quarantine(&client, vec![&client2], "after starting client2");
}

// PS: trusted as in poldercast-trusted, not trusted peer
#[test]
pub fn node_does_not_quarantine_trusted_node() {
    let mut network_controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(SERVER))
                .with_node(Node::new(CLIENT).with_trusted_peer(SERVER)),
        )
        .blockchain_config(Blockchain::default().with_leader(SERVER))
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(1_000_000)
                .delegated_to(CLIENT)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(1_000_000)
                .delegated_to(SERVER)
                .build(),
        )
        .build()
        .unwrap();

    let server = network_controller
        .spawn(SpawnParams::new(SERVER).in_memory())
        .unwrap();
    let client = network_controller
        .spawn(SpawnParams::new(CLIENT).in_memory())
        .unwrap();

    utils::wait(5);

    assert_node_stats(&server, 1, 0, 1, "before stopping client");
    assert_empty_quarantine(&server, "before stopping client");

    client.shutdown();
    utils::wait(20);

    // The server "forgets" the client but does not quarantine it
    assert_node_stats(&server, 1, 0, 1, "before restarting client");
    assert_empty_quarantine(&server, "before restarting client");
}
