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
$ jormungandr generate-priv-key --type=Ed25519
ed25519_secret1sx3vjuc733mmlfm86xc2ufrjx6hltas57yj6pklra3s2n03yfv2svckr67
```

and to extract the associated public key:

```
$ echo ed25519_secret1sx3vjuc733mmlfm86xc2ufrjx6hltas57yj6pklra3s2n03yfv2svckr67 | jormungandr generate-pub-key
ed25519_public1zkt4e7yufvf4es45u4wc0kztzkfr0n3wa6ks04z6kltsxdalafxqp7m2ca
```
