
#[test]
pub fn node_whitelist_itself() {
    let mut network_controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(SERVER))
                .with_node(Node::new(CLIENT).with_trusted_peer(SERVER)),
        )
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
                .with_node(Node::new(CLIENT).with_trusted_peer(SERVER)),
        )
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

    let fake_addr = "/ip4/127.0.0.1/tcp/80";
    let policy = Policy {
        quarantine_duration: Some(Duration::new(30, 0)),
        quarantine_whitelist: Some(vec![fake_addr.parse().unwrap()]),
    };

    let server = network_controller
        .spawn(SpawnParams::new(SERVER).policy(policy))
        .unwrap();

    let _client = network_controller
        .spawn(
            SpawnParams::new(CLIENT)
                // The client broadcast a different ip address from the one it's actually
                // listening to, so that the server will fail connection
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

    assert_node_stats(&server, 1, 0, 1, "after starting client");
    assert_empty_quarantine(&server, "after starting client");
}

#[test]
pub fn node_put_in_quarantine_nodes_which_are_not_whitelisted() {
    let mut network_controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(SERVER))
                .with_node(Node::new(CLIENT).with_trusted_peer(SERVER)),
        )
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
        .spawn(
            SpawnParams::new(CLIENT)
                // The client broadcast a different ip address from the one it's actually
                // listening to, so that the server will fail connection and put it in quarantine
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

    process_utils::sleep(5);

    assert_node_stats(&server, 0, 1, 1, "after starting client");
    assert_are_in_quarantine(&server, vec![&client], "after starting client");
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

    process_utils::sleep(5);

    assert_node_stats(&server, 1, 0, 1, "before stopping client");
    assert_empty_quarantine(&server, "before stopping client");

    client.shutdown();
    process_utils::sleep(20);

    // The server "forgets" the client but does not quarantine it
    assert_node_stats(&server, 1, 0, 1, "before restarting client");
    assert_empty_quarantine(&server, "before restarting client");
}
