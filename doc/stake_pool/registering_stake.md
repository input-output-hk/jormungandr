# registering your stake

This is the primary operation to do on the blockchain: declare
your stake key. This is the key that will allow you to **group**
your funds and claim stake.

You will need this in order to participate to the proof of stake
protocol (via delegating or owning a stake pool).

## You stake key pair

This is the key pair that will identify you as a stake owner in the
blockchain. It is preferable to use a key pair that is different from
your wallet (for security reason). See the [`jcli key`] documentation
to generate a new key pair of type `Ed25519` or `Ed25519Extended`. For example:

```
$ jcli key generate --type=Ed25519Extended > stake_key.prv
```

The file `stake_key.prv` will contain your private key.

```
$ cat stake_key.prv | jcli key to-public > stake_key.pub
```

The file `stake_key.pub` will contain your public key.
