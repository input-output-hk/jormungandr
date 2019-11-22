This chapter covers the node documentation, necessary to have a working system. It covers
the network, logging and storage parameters.

## Node Configuration
This is an common example of a Jormungandr node configuration file typically named `node-config.yaml`, however your's will vary depending on your needs.  Addtionally, this configuration has been tested on Jormungandr 0.7.1 and may change with newer versions.  It's important to keep in mind that the trusted_peers portion of this configuration will be different for each Cardano blockchain network.  If you're trying to connect this node to a specific network, you need to know it's genesis block hash, and it's associated list of trusted peers.

Example Configuration - 1:

```YAML
---
log:
  - output: stderr
    level:  info
    format: plain
p2p:
  public_address: "/ip4/40.90.149.161/tcp/3200" # This should match your public IP address.     
  topics_of_interest:
    blocks: normal #Default is normal - high for stakepool
    messages: low   #Default is low - high for stakepool
  trusted_peers:
  - address: "/ip4/13.230.137.72/tcp/3000"
    id: e4fda5a674f0838b64cacf6d22bbae38594d7903aba2226f
  - address: "/ip4/13.230.48.191/tcp/3000"
    id: c32e4e7b9e6541ce124a4bd7a990753df4183ed65ac59e34
  - address: "/ip4/18.196.168.220/tcp/3000"
    id: 74a9949645cdb06d0358da127e897cbb0a7b92a1d9db8e70
  - address: "/ip4/3.124.132.123/tcp/3000"
    id: 431214988b71f3da55a342977fea1f3d8cba460d031a839c
  - address: "/ip4/18.184.181.30/tcp/3000"
    id: e9cf7b29019e30d01a658abd32403db85269fe907819949d
  - address: "/ip4/184.169.162.15/tcp/3000"
    id: acaba9c8c4d8ca68ac8bad5fe9bd3a1ae8de13816f40697c
  - address: "/ip4/13.56.87.134/tcp/3000"
    id: bcfc82c9660e28d4dcb4d1c8a390350b18d04496c2ac8474
rest:
  listen: 127.0.0.1:3100
storage: "./storage"
```
Note:
  The node configuration uses the [YAML](https://en.wikipedia.org/wiki/YAML) format.
