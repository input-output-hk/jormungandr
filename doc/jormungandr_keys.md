# Jormungandr Keys

There are multiple type of key for multiple reasons.

| type | usage |
|:----:|:------|
|`Ed25519` | Signing algorithm for Ed25519 algorithm |
|`Ed25519Bip32`| For HDWallet, this type of keys allow deterministic derivation |
|`Ed25519Extended`| Compatible with `Ed25519Bip32` but without the derivation |
|`FakeMMM`| For stake pool, necessary for the KES |
|`Curve25519_2HashDH`| For stake pool, necessary for the VRF |

There is a command line parameter to generate this keys:

```
$ jcli key generate --type=Ed25519
ed25519_sk1cvac48ddf2rpk9na94nv2zqhj74j0j8a99q33gsqdvalkrz6ar9srnhvmt
```

and to extract the associated public key:

```
$ echo ed25519_sk1cvac48ddf2rpk9na94nv2zqhj74j0j8a99q33gsqdvalkrz6ar9srnhvmt | jcli key to-public
ed25519_pk1z2ffur59cq7t806nc9y2g64wa60pg5m6e9cmrhxz9phppaxk5d4sn8nsqg
```
