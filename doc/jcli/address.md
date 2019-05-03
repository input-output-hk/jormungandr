# address

Jormungandr comes with a separate CLI to create and manipulate addresses.

This is useful for creating addresses from public keys in the CLI,
for debugging addresses and for testing.

## display address info

To display an address and verify it is in a valid format you can utilise:

```
$ jcli address info ta1svy0mwwm7mdwcuj308aapjw6ra4c3e6cygd0f333nvtjzxg8ahdvxlswdf0
discrimination: testing
public key: ed25519e_pk1pr7mnklkmtk8y5tel0gvnksldwywwkpzrt6vvvvmzus3jpldmtpsx9rnmx
```

or for example:

```
$ jcli address \
    info \
    ca1qsy0mwwm7mdwcuj308aapjw6ra4c3e6cygd0f333nvtjzxg8ahdvxz8ah8dldkhvwfghn77se8dp76uguavzyxh5cccek9epryr7mkkr8n7kgx
discrimination: production
public key: ed25519e_pk1pr7mnklkmtk8y5tel0gvnksldwywwkpzrt6vvvvmzus3jpldmtpsx9rnmx
group key:  ed25519e_pk1pr7mnklkmtk8y5tel0gvnksldwywwkpzrt6vvvvmzus3jpldmtpsx9rnmx
```

## Creating an address

every of the command below allows to create address for production or for testing.
This is for discrimination of addresses and to prevent users to send funds when utilising
a testnet environment. To create an address for testing simply add the flag `--testing`.

### Address for UTxO

You can create a bootstrap era address utilising the following command.

```
$ jcli address \
    single ed25519e_pk1jnlhwdgzv3c9frknyv7twsv82su26qm30yfpdmvkzyjsdgw80mfqduaean
ca1qw207ae4qfj8q4yw6v3ned6psa2r3tgrw9u3y9hdjcgj2p4pcaldyukyka8
```

This kind of address are useful when running in the BFT era or if delegation is not
desired.

To add the delegation, simply add the delegation key as a second parameter of the command:

```
$ jcli address \
    single \
    ed25519e_pk1fxvudq6j7mfxvgk986t5f3f258sdtw89v4n3kr0fm6mpe4apxl4q0vhp3k \
    ed25519e_pk1as03wxmy2426ceh8nurplvjmauwpwlcz7ycwj7xtl9gmx9u5gkqscc5ylx
ca1q3yen35r2tmdye3zc5lfw3x992s7p4dcu4jkwxcda80tv8xh5ym74mqlzudkg42443nw08cxr7e9hmcuzals9ufsa9uvh723kvteg3vpvrcxcq
```

### Address for Account

Account are much simpler to utilise, they are needed to create reward account
but it is also possible to utilise them as a wallet.

To create an account:

```
$ jcli address \
    account ed25519e_pk1c4yq3hflulynn8fef0hdq92579n3c49qxljasrl9dnuvcksk84gs9sqvc2
ca1qhz5szxa8lnujwva8997a5q42nckw8z55qm7tkq0u4k03nz6zc74ze780qe
```
