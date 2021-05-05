# Scenario Tests

## Interactive

prepares small network of nodes and allows to interact with them, by sending transactions, utilize some query calls, bootstrap nodes or shut it down

### How to run
```
cd testing/jormungandr-scenario-tests
cargo run --bin interactive
```

## Vit Backend

### Build & Dependencies

#### Prereqs

Before running vit backed all dependencies need to be build:

```
cd jormungandr

cargo install --locked --force --path jormungandr

cargo install --locked --force --path jcli

cargo install --locked --force --path testing/iapyx

cargo install --locked --force --path testing/jormungandr-scenario-tests
```

#### Run

example of simplest backend bootstrap, which starts voting after 20 minutes from end of setup:

```
vit start quick
```

in above case all data will be put in : `./vit-backend` folder including all qr codes and private keys

see more parameters using command:

```
vit --help
```

##### Modes

There are two modes, which can be used:

- interactive : allows to interact with backend, by sending transactions, tally the votes and utilize some query calls
- monitor: idle mode with live output on console


## Scenario tests
Multi node scenarios, which aim to test nodes behaviour in presence of other nodes or within given network topologies for particular network settings or in occurence on some node disruption


### How to run functional tests
```
cd jormungandr-scenario-tests
cargo run -- --tag short
```

### How to run real network tests
```
cd jormungandr-scenarios-tests
cargo run -- --scenario real_network
```
