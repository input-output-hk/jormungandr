# Address

Jormungandr comes with a separate CLI to create and manipulate addresses.

This is useful for creating addresses from their components in the CLI,
for debugging addresses and for testing.

## Display address info

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
public key: ed25519_pk1pr7mnklkmtk8y5tel0gvnksldwywwkpzrt6vvvvmzus3jpldmtpsx9rnmx
group key:  ed25519_pk1pr7mnklkmtk8y5tel0gvnksldwywwkpzrt6vvvvmzus3jpldmtpsx9rnmx
```

## Creating an address

Each command following allows to create addresses for production and testing
chains. For chains, where the discrimination is `testing`, you need to
use the `--testing` flag.

There's 3 types of addresses:

* Single address : A simple spending key. This doesn't have any stake in the system
* Grouped address : A spending key attached to an account key. The stake is automatically
* Account address : An account key. The account is its own stake

### Address for UTxO

You can create a single address (non-staked) using the spending public key for
this address utilising the following command:

```
$ jcli address \
    single ed25519e_pk1jnlhwdgzv3c9frknyv7twsv82su26qm30yfpdmvkzyjsdgw80mfqduaean
ca1qw207ae4qfj8q4yw6v3ned6psa2r3tgrw9u3y9hdjcgj2p4pcaldyukyka8
```

To add the staking information and make a group address, simply add the account
public key as a second parameter of the command:

```
$ jcli address \
    single \
    ed25519_pk1fxvudq6j7mfxvgk986t5f3f258sdtw89v4n3kr0fm6mpe4apxl4q0vhp3k \
    ed25519_pk1as03wxmy2426ceh8nurplvjmauwpwlcz7ycwj7xtl9gmx9u5gkqscc5ylx
ca1q3yen35r2tmdye3zc5lfw3x992s7p4dcu4jkwxcda80tv8xh5ym74mqlzudkg42443nw08cxr7e9hmcuzals9ufsa9uvh723kvteg3vpvrcxcq
```

### Address for Account

To create an account address you need the account public key and run:

```
$ jcli address \
    account ed25519_pk1c4yq3hflulynn8fef0hdq92579n3c49qxljasrl9dnuvcksk84gs9sqvc2
ca1qhz5szxa8lnujwva8997a5q42nckw8z55qm7tkq0u4k03nz6zc74ze780qe
```
