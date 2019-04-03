# address CLI

Jormungandr comes with a separate CLI to create and manipulate addresses.

This is useful for creating addresses from public keys in the CLI,
for debugging addresses and for testing.

## display address info

To display an address and verify it is in a valid format you can utilise:

```
$ jcli address info ta1svy0mwwm7mdwcuj308aapjw6ra4c3e6cygd0f333nvtjzxg8ahdvxlswdf0
discrimination: testing
public key: ed25519extended_public1pr7mnklkmtk8y5tel0gvnksldwywwkpzrt6vvvvmzus3jpldmtps3t9h3a
```

or for example:

```
$ jcli address \
    info \
    ca1qsy0mwwm7mdwcuj308aapjw6ra4c3e6cygd0f333nvtjzxg8ahdvxz8ah8dldkhvwfghn77se8dp76uguavzyxh5cccek9epryr7mkkr8n7kgx
discrimination: production
public key: ed25519extended_public1pr7mnklkmtk8y5tel0gvnksldwywwkpzrt6vvvvmzus3jpldmtps3t9h3a
group key:  ed25519extended_public1pr7mnklkmtk8y5tel0gvnksldwywwkpzrt6vvvvmzus3jpldmtps3t9h3a
```

## Creating an address

every of the command below allows to create address for production or for testing.
This is for discrimination of addresses and to prevent users to send funds when utilising
a testnet environment. To create an address for testing simply add the flag `--testing`.

### Address for UTxO

You can create a bootstrap era address utilising the following command.

```
$ jcli address \
    single ed25519extended_public1pr7mnklkmtk8y5tel0gvnksldwywwkpzrt6vvvvmzus3jpldmtps3t9h3a
ca1qvy0mwwm7mdwcuj308aapjw6ra4c3e6cygd0f333nvtjzxg8ahdvx5c3cy4
```

This kind of address are useful when running in the BFT era or if delegation is not
desired.

To add the delegation, simply add the delegation key as a second parameter of the command:

```
$ jcli address \
    single \
    ed25519extended_public1pr7mnklkmtk8y5tel0gvnksldwywwkpzrt6vvvvmzus3jpldmtps3t9h3a \
    ed25519extended_public13talprd9grgaqzs42mkm0x2xek5wf9mdf0eefdy8a6dk5grka2gstrp3en
ca1qsy0mwwm7mdwcuj308aapjw6ra4c3e6cygd0f333nvtjzxg8ahdv8zhm7zx62sx36q9p24hdk7v5dndgujtk6jlnjj6g0m5mdgs8d653lpq5dq
```

### Address for Account

Account are much simpler to utilise, they are needed to create reward account
but it is also possible to utilise them as a wallet.

To create an account:

```
$ jcli address \
    account ed25519extended_public1pr7mnklkmtk8y5tel0gvnksldwywwkpzrt6vvvvmzus3jpldmtps3t9h3a
ca1q5y0mwwm7mdwcuj308aapjw6ra4c3e6cygd0f333nvtjzxg8ahdvx6g5gwu
```
