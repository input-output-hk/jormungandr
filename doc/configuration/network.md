
There's 2 differents network interfaces which are covered by their respective section:

```
rest:
   ...
peer_2_peer:
   ...
```

## REST interface configuration

- *listen*: listen address
- *pkcs12*: certificate file (optional)
- *prefix*: (optional) api prefix

## P2P configuration

- *trusted_peers*: (optional) the list of nodes to connect to in order to
    bootstrap the p2p topology (and bootstrap our local blockchain);
- *public_id*: (optional) the public identifier send to the other nodes in the
    p2p network. If not set it will be randomly generated.
- *public_address*: the address to listen from and accept connection
    from. This is the public address that will be distributed to other peers
    of the network that may find interest into participating to the blockchain
    dissemination with the node;
- *topics_of_interests*: the different topics we are interested to hear about:
    - *messages*: notify other peers this node is interested about Transactions
    typical setting for a non mining node: `"low"`. For a stakepool: `"high"`;
    - *blocks*: notify other peers this node is interested about new Blocs.
    typical settings for a non mining node: `"normal"`. For a stakepool: `"high"`;