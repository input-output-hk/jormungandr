# Full Node

> Just because you call something a blockchain, that doesn't mean you aren't subject to normal engineering laws.

User guide documentation available [here](https://input-output-hk.github.io/jormungandr)

## Master current build status

| CI | Status | Description |
|---:|:------:|:------------|
| Travis CI | [![Build Status](https://travis-ci.org/input-output-hk/jormungandr.svg?branch=master)](https://travis-ci.org/input-output-hk/jormungandr) | Master and release |
| CircleCI | [![CircleCI](https://circleci.com/gh/input-output-hk/jormungandr/tree/master.svg?style=svg)](https://circleci.com/gh/input-output-hk/jormungandr/tree/master) | Master and PRs |
| Appveyor | [![Build status](https://ci.appveyor.com/api/projects/status/1y5583gqc4xn8x3j/branch/master?svg=true)](https://ci.appveyor.com/project/NicolasDP/jormungandr/branch/master) | Master, release and PRs |

## Install from Binaries

Use the [Latest Binaries](https://github.com/input-output-hk/jormungandr/releases), available
for many operating systems and architectures.

## Install from Source

### Prerequisites

[Rust Compiler](https://www.rust-lang.org/tools/install) (latest stable version is recommended, minimum required: 1.35+)


```sh
rustup install stable
rustup default stable
```


#### Path

* Win: Add `%USERPROFILE%\.cargo\bin` to the  environment variable `PATH`.
* Lin/Mac: Add `${HOME}/.cargo/bin` to your `PATH`.

#### cc, protobuf

* Make sure the C compiler toolchain is installed and, on Unix (e.g. macOS),
  the compiler and linker executable `cc` is found in `PATH`.
* On Linux with systemd: to enable logging to journald replace step 9
  with `cargo install --path jormungandr --features systemd`.
* The build requires the [Protocol Buffers][protobuf] compiler:
  - On Linux environments without glibc such as Alpine, the protobuf compiler
    `protoc` needs to be installed and found in `PATH` or otherwise
    specified in the environment variable `PROTOC`.
  - For distribution or container builds in general, it's a good practice to
    install `protoc` from the official distribution package if available,
    otherwise the version bundled with crate `prost-build` will be used.
  - **NixOS** users should rely on [shell.nix](shell.nix) provided in this source
    tree to pull the dependencies and set up the environment for the build.


[protobuf]: https://developers.google.com/protocol-buffers/

### Commands

Check `<latest release tag>` on https://github.com/input-output-hk/jormungandr/releases/latest

```sh
git clone --recurse-submodules https://github.com/input-output-hk/jormungandr
cd jormungandr
git checkout tags/<latest release tag> #replace this with something like v1.2.3
git submodule update
cargo install --path jormungandr # --features # systemd (on linux with systemd)
cargo install --path jcli
```


This will install 2 tools:

* `jormungandr`: the node part of the blockchain;
* `jcli`: a command line helper tool to help you use and setup the node;


## How To Use

A functional node needs 2 configurations:

1. Its own system configuration: Where to store data, network configuration, logging.
2. The blockchain genesis configuration which contains the initial trusted setup of the blockchain:
   coin configuration, consensus settings, initial state.

In normal use, the blockchain genesis configuration is given to you or
automatically fetched from the network.

More documentation on the node configuration can be found [here](https://input-output-hk.github.io/jormungandr/configuration/introduction.html),
and for the blockchain genesis configuration [here](https://input-output-hk.github.io/jormungandr/advanced/introduction.html)

## Quick-Start for private mode

Follow instructions on installation, then to start a private and minimal
test setup:

1. In terminal, create an empty directory somewhere and enter this directory
2. `PATH/TO/SOURCE/REPOSITORY/scripts/bootstrap <options>`
3. execute the instruction to start printed at the end

For a BFT setup, use the following recommended options:

    bootstrap -b

For a Genesis-praos setup, use the following recommended options:

    bootstrap -g -s 2

For help on the options:

    bootstrap -h

The bootstrap script creates a simple setup with a faucet with 10 millions
coins, a BFT leader, and a stake pool.

The bootstrap script also create 2 shell scripts parametrized to this specific
run of bootstrap:

* `faucet-send-money`
* `faucet-send-certificate`

Both scripts can be used to do simple limited operation through the jcli debugging tools.

## Quick-Start in public mode
With release of 0.6.0, public mode became available; there are currently two testnets operating at any given time:
- beta testnet
- nightly testnet
 
To start a new node from scratch on a given blockchain, you need to know the
block0 hash of this blockchain for trust purpose and internet peers to connect
to. The simplest way to start such a node is:

    jormungandr --block0-hash <HASH> --trusted-peers <IPs>
    
In order to connect your node to a IOHK operated beta testnet, [follow the official documentation](https://testnet.iohkdev.io/cardano/shelley/). In order to connect to a nightly testnet, it's best to seek support in [Cardano Stake Pool Workgroup Telegram group](https://web.telegram.org/#/im?p=@CardanoStakePoolWorkgroup).

## Documentation

Documentation is available in the markdown format [here](doc/SUMMARY.md)

## License

This project is licensed under either of the following licenses:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)
