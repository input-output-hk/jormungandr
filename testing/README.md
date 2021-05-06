# Tests Collection

## Api tests
Api and unit tests in `chain-impl-mockchain`.
Tests are covering internals of ledger library, validating particular modules (stake distribution).
There are also scenarios for testing ledger in a 'turn-based' aproach. For example:

```
    /// Prepare blockchain settings and actors
    let (mut ledger, controller) = prepare_scenario()
        .with_config(
            ConfigBuilder::new(0)
                .with_discrimination(Discrimination::Test)
                .with_fee(LinearFee::new(1, 1, 1)),
        )
        .with_initials(vec![
            wallet("Alice").with(1_000).owns("alice_stake_pool"),
            wallet("Bob").with(1_000).owns("bob_stake_pool"),
            wallet("Clarice").with(1_000).owns("clarice_stake_pool"),
            wallet("David").with(1_003),
        ])
        .build()
        .unwrap();

    /// Retrieve actors
    let alice_stake_pool = controller.stake_pool("alice_stake_pool").unwrap();
    let bob_stake_pool = controller.stake_pool("bob_stake_pool").unwrap();
    let clarice_stake_pool = controller.stake_pool("clarice_stake_pool").unwrap();

    let david = controller.wallet("David").unwrap();

    // prepare delegation ratio
    let delegation_ratio = vec![
        (&alice_stake_pool, 2u8),
        (&bob_stake_pool, 3u8),
        (&clarice_stake_pool, 5u8),
    ];

    /// post delegation certificates
    controller
        .delegates_to_many(&david, &delegation_ratio, &mut ledger)
        .unwrap();

    /// verify distribution is correct
    let expected_distribution = vec![
        (alice_stake_pool.id(), Value(200)),
        (bob_stake_pool.id(), Value(300)),
        (clarice_stake_pool.id(), Value(500)),
    ];

    LedgerStateVerifier::new(ledger.clone().into())
        .info("after delegation to many stake pools")
        .distribution()
        .pools_distribution_is(expected_distribution);

```


### How to run tests
```
cd chain-deps
cargo test
```

### Frequency
Tests are run on each PR


## Integration tests
End to end tests for self-node and jcli. Using rest api and jcli tests are validating node correctness, stability and interaction with database/rest api. Also there are non-functional tests which verify node durability and reliability

### How to run tests functional tests
```
cd jormungandr-integration-tests
cargo test
```

### How to run performance tests
```
cd jormungandr-integration-tests
cargo test non_functional --feature sanity-non-functional
```

### How to run testnet soak tests
```
cd jormungandr-integration-tests
cargo test non_unctional --feature soak-non-functional
```

### Frequency
Functional tests are run on each PR. Performance and testnet integration tests are run nightly

## Scenario tests
Multi node scenarios, whcich aim to test nodes behaviour in presence of other nodes or within given network topologies for particular network settings or in occurence on some node disruption


### How to run functional tests
```
cd jormungandr-scenarios-tests
cargo run -- --tag short
```

### How to run real network tests
```
cd jormungandr-scenarios-tests
cargo run -- --scenario real_network
```

# Performance tests dashboard

https://cardano-rust-testrun-logs.s3.eu-central-1.amazonaws.com/performance_dashboard.html

