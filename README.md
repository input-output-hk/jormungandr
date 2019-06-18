# Full Node

> Just because you call something a blockchain, that doesn't mean you aren't subject to normal engineering laws.

User guide documentation available [here](https://input-output-hk.github.io/jormungandr)

## How to install from sources

Currently the minimum supported version of the rust compiler is 1.35, however
we recommend to use the most recent stable version of the rust compiler.

1. [install rustup](https://www.rust-lang.org/tools/install)
2. Run `rustup install stable`
3. Run `rustup default stable`
4. Clone this repository: `git clone --recurse-submodules https://github.com/input-output-hk/jormungandr`
5. Enter the repository directory: `cd jormungandr`
5. install **jormungandr**: `cargo install --path jormungandr`
6. install **jcli**: `cargo install --path jcli`

Note:

* on windows, you'll need to add the `/userProfile/.cargo/bin` into the Path;
* on linux and OSX: add `${HOME}/.cargo/bin` to your `${PATH}`
* on linux with systemd: to enable logging to journald replace step 5. with `cargo install --path . --features systemd`

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

### Starting the node

If you are not a leader node, then you can start the jormundandr with:

```sh
jormungandr --genesis-block block-0.bin \
  --config example.config
```

# Documentation

Documentation is available in the markdown format [here](doc/SUMMARY.md)

# License

This project is licensed under either of the following licenses:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)
