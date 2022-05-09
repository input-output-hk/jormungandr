# jormungandr-automation

Incubator of all testing apis for the node and jcli:

## build

In order to build jormungandr-automation in main project folder run:
```
cd testing/jormungandr-automation
cargo build
```

## jcli testing api

Api that can be used to run jcli executable underneath and is capable to assert outcome of command. It can work with already installed jcli (using PATH variable) or custom path. For Example:

```
    let jcli: JCli = Default::default();
    let private_key = jcli.key().generate("Ed25519Extended");
    let public_key = jcli.key().convert_to_public_string(&private_key);
```

## jormungandr testing api

Collection of automation modules for node interaction and configuration:

* configuration - allows to configure node & blockchain settings,
* explorer - explorer configuration/bootstrap & interaction module,
* grpc - module for grpc internode connection library handling. capable of sending some RPC calls as well as bootstrap receiver instance,
* legacy - module for loosely typed configuration. This allow to bootstrap older version of node, for example to satisfy need on cross-version testing,
* rest - module for jormungandr REST api testing,
* starter - module for bootstraping node,
* verifier - node state verifier
* logger - api for jormungandr log handling/assertion
* process - singleton class for handling jormungandr process

## testing

Bunch of losely coupled utility modules, mostly for additional configuration capabilities or benchmarking.
