# Thor

Thor is a wallet cli & wallet api project which operates on jormungandr network.

WARNING: main purpose of the wallet is testing. Do NOT use it on production.

## Build & Install

In order to build hersir in main project folder run:
```
cd testing/hersir
cargo build
cargo install --path . --force
```

## Quick Start

### CLI

Thor can be used as a wallet cli. It is capable of sending transactions or pull data from node. The simplest usage example is available by using commands:


* register new wallet based on secret key:
`thor wallets import --alias darek --password 1234 secret.file`

* connect to node rest API:
`thor connect https://jormungandr.iohk.io/api`

* use recently created wallet for rest of commands:
`thor wallets use darek`

* sync with the node regarding wallet data:
`thor wallets refresh`

* send transaction:
`thor send tx --ada 5 --address ca1q5srhkdfuxqdm6h57mj45acxcdr57cr5lhddzkrjqyl8mmw62v9qczh78cu -pin 1234`

### API

Thor also allows you to use it as Api to perform any wallet operations from the code:

```
    use thor::{Wallet, FragmentSender, FragmentSenderSetup, FragmentVerifier};

    let receiver = thor::Wallet::default();
    let mut sender = thor::Wallet::default();

    // node bootstrap
    let jormungandr = ...

    let fragment_sender = FragmentSender::from_with_setup(
        jormungandr.block0_configuration(),
        FragmentSenderSetup::no_verify(),
    );

    fragment_sender
        .send_transaction(&mut sender, &receiver, &jormungandr, 1.into())
        .unwrap();

```

## Configuration

Thor api doesn't use any configuration files. However cli uses small cache folder on filesystem (located in: `~/.thor`).
The purpose of this configuration is to store wallet lists as well as secret keys guarded by pass phrase.

### full list of available commands

Full list of commands is available on `thor --help` command.

```
thor 0.1.0
Command line wallet for testing Jormungandr

USAGE:
    thor <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    address                 Gets address of wallet in bech32 format
    clear-tx                Clears pending transactions to confirm. In case if expiration occured
    confirm-tx              Confirms successful transaction
    connect                 Sets node rest API address. Verifies connection on set
    help                    Prints this message or the help of the given subcommand(s)
    logs                    Prints entire fragment logs from the node
    pending-transactions    Prints pending transactions (not confirmed)
    refresh                 Pulls wallet data from the node
    send                    Sends fragments to nodes
    status                  Prints wallet status (balance/spending counters/tokens)
    statuses                Prints pending or already sent fragments statuses
    wallets                 Allows to manage wallets: add/remove/select operations
```
