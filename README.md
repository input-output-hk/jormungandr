# Full Node

> Just because you call something a blockchain, that doesn't mean you aren't subject to normal engineering laws.

User guide documentation available [here](https://input-output-hk.github.io/jormungandr)

## Master current build status

| CI | Status | Description |
|---:|:------:|:------------|
| Travis CI | [![Build Status](https://travis-ci.org/input-output-hk/jormungandr.svg?branch=master)](https://travis-ci.org/input-output-hk/jormungandr) | Master and release |
| CircleCI | [![CircleCI](https://circleci.com/gh/input-output-hk/jormungandr/tree/master.svg?style=svg)](https://circleci.com/gh/input-output-hk/jormungandr/tree/master) | Master and PRs |
| Appveyor | [![Build status](https://ci.appveyor.com/api/projects/status/1y5583gqc4xn8x3j/branch/master?svg=true)](https://ci.appveyor.com/project/NicolasDP/jormungandr/branch/master) | Master, release and PRs |

## How to install from sources

Currently the minimum supported version of the rust compiler is 1.35, however
we recommend to use the most recent stable version of the rust compiler.

1. [Install rustup](https://www.rust-lang.org/tools/install)
2. Run `rustup install stable`
3. Run `rustup default stable`
4. Clone this repository: `git clone --recurse-submodules https://github.com/input-output-hk/jormungandr`
5. Enter the repository directory: `cd jormungandr`
6. Check latest release tag on https://github.com/input-output-hk/jormungandr/releases/latest
7. checkout this tag ` git checkout tags/<latest release tag>`
8. install **jormungandr**: `cargo install --path jormungandr`
9. install **jcli**: `cargo install --path jcli`

Note:

* On Windows, you'll need to add the `%USERPROFILE%\.cargo\bin` into the
  environment variable `PATH`.
* On Linux and macOS: add `${HOME}/.cargo/bin` into your `PATH`.
* Make sure the C compiler toolchain is installed and, on Unix (e.g. macOS),
  the compiler and linker executable `cc` is found in `PATH`.
* On Linux with systemd: to enable logging to journald replace step 6
  with `cargo install --path . --features systemd`.
* The build requires the [Protocol Buffers][protobuf] compiler:
  - On Linux environments without glibc such as Alpine, the protobuf compiler
    `protoc` needs to be installed and found in `PATH` or otherwise
    specified in the environment variable `PROTOC`.
  - NixOS users should rely on [shell.nix](shell.nix) provided in this source
    tree to pull the dependencies and set up the environment for the build.
  - For distribution or container builds in general, it's a good practice to
    install `protoc` from the official distribution package if available,
    otherwise the version bundled with crate `prost-build` will be used.

[protobuf]: https://developers.google.com/protocol-buffers/

This will install 2 tools:

* `jormungandr`: the node part of the blockchain;
* `jcli`: a command line helper tool to help you use and setup the node;

## How to install from binaries

Our binaries releases are available [here](https://github.com/input-output-hk/jormungandr/releases)
for many operating systems and architecture, but in due time, jormungandr will
be available through package managers.

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

:warning: This is not currently functional :warning:

To start a new node from scratch on a given blockchain, you need to know the
block0 hash of this blockchain for trust purpose and internet peers to connect
to. The simplest way to start such a node is:

    jormungandr --block0-hash <HASH> --trusted-peers <IPs>

# Documentation

Documentation is available in the markdown format [here](doc/SUMMARY.md)

# License

This project is licensed under either of the following licenses:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)
