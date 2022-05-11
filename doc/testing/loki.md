# Loki

Loki is an adversary node implementation and api which operates on jormungandr network.

## Build & Install

In order to build hersir in main project folder run:
```
cd testing/loki
cargo build
cargo install --path . --force
```

## Quick Start

Loki can be used bootstrap using cli:


```
loki --genesis-block block0.bin --listen-address 127.0.0.1:8080 -s secret.yaml
```

where:

`genesis-block` - Path to the genesis block (the block0) of the blockchain
`listen-address` - Specifies the address the node will listen
`secret` - Set the secret node config (in YAML format). Example:
```
---
bft:
    signing_key: ed25519_sk1w2tyr7e2w26w5vxv65xf36kpvcsach8rcdmlmrhg3rjzeumjnzyqvdvwfa
```

Then utilizing rest interface of loki node one can send some invalid GRPC messages to rest of the network:

```
curl --location --request POST 'http://127.0.0.1:8080/invalid_fragment' \
--header 'Content-Type: application/json' \
--data-raw '{
    "address": "127.0.0.1:1000",
    "parent": "tip"
}'
```

where:

`address` - address of "victim" node,
`parent` - Parent block. Possible values:
* `tip` - current tip of "victim" node,
* `block0` - block0,
* `{Hash}` - arbitrary parent block which hash is provided in request

#### Other possible operations:

* `/invalid_hash` - Sends block with invalid hash,
* `/invalid_signature` - Sends block with invalid by wrong leader signature,
* `/nonexistent_leader` - Sends block with non-existing leader,
* `/wrong_leader` - Sends block with signed with invalid leader,

### API

Loki also provides API for performing adversary operations, like sending invalid fragments:

```
    use loki::{AdversaryFragmentSender, AdversaryFragmentSenderSetup};

    let mut sender = ...
    let receiver = ..

    // node initialization
    let jormungandr = ...

    let adversary_sender = AdversaryFragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        BlockDate::first().next_epoch().into(),
        AdversaryFragmentSenderSetup::no_verify(),
    );

    adversary_sender
        .send_faulty_transactions_with_iteration_delay(
            10,
            &mut sender,
            &receiver,
            &jormungandr,
            Duration::from_secs(5),
        )
        .unwrap();
```
