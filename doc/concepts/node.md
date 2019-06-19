# Node organisation

## Node Files
<mention files that the node needs to start: genesis, config.yaml> 

## Secure Enclave

The secure enclave is the component containing the secret cryptographic
material, and offering safe and secret high level interfaces to the rest of
the node.

## Network

The node's network is 3 components:

* Intercommunication API (GRPC)
* Public client API (REST)
* Control client API (REST)

### Intercommunication API (GRPC)

This interface is a binary, efficient interface using the protobuf format and
GRPC standard. The protobuf files of types and interfaces are available in
the source code.

The interface is responsible to communicate with other node in the network:

* block sending and receiving
* fragments (transaction, certificates) broadcast
* peer2peer gossip

### Public API REST

This interface is for simple queries for clients like:

* Wallet Client & Middleware
* Analytics & Debugging tools
* Explorer

it's recommended for this interface to not be opened to the public.

TODO: Add a high level overview of what it does

### Control API REST

This interface is not finished, but is a restricted interface with ACL,
to be able to do maintenance tasks on the process:

* Shutdown
* Load/Retire cryptographic material

TODO: Detail the ACL/Security measure

## Logging Considerations
<include some logging considerations>
