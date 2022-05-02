
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
cargo test non_functional --features sanity-non-functional
```

### How to run testnet soak tests
```
cd jormungandr-integration-tests
cargo test non_unctional --features soak-non-functional
```

### Frequency
Functional tests are run on each PR. Performance and testnet integration tests are run nightly

## Scenario tests
Multi node scenarios, whcich aim to test nodes behaviour in presence of other nodes or within given network topologies for particular network settings or in occurence on some node disruption


### How to run network tests
```
cd jormungandr-integration-tests
cargo test --features network
```

### How to run performance network tests
```
cd jormungandr-integration-tests
cargo test --features network-non-functional
```

# Performance tests dashboard

https://cardano-rust-testrun-logs.s3.eu-central-1.amazonaws.com/performance_dashboard.html

