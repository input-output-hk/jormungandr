# Thor

Thor is a testing wallet implementation. It is mostly meant for testers and dev-ops to easily
interact with Jormungandr blockchain. It allows operations:

* importing cryptographic materials for the wallets and basic wallet management.
* sending transactions and varius fragments
* preserving wallet state
* providing minimal authorization over wallet secret key


## Api

Main responsibility of thor is to provide testing api for wallet and fragment sender capabilities.
For example:

```
    use thor::{BlockDateGenerator, FragmentSender, FragmentSenderSetup, StakePool, Wallet};

    /// creating account type address
    let mut alice = thor::Wallet::default();
    let mut bob = thor::Wallet::default();

    ...

    /// Sending fragments to a node

    let transaction_sender = thor::FragmentSender::new(
        node.genesis_block_hash(),
        node.fees(),
        //expiry block date
        BlockDate {
            epoch: 10,
            slot_id: 0,
        }.into(),
        thor::FragmentSenderSetup::resend_3_times(),
    );

    transaction_sender
        .send_transaction(
            &mut alice,
            &bob,
            &node,
            10.into(),
        )
        .unwrap();

```

## Cli

For tester convenience, thor also provides cli, which can be used as wallet app for testing purposes.

For example:

### 1. Importing new wallet:

```
thor wallets import --password 1234 --alias Dariusz .\secret.key
```

### 2. Setting wallet as default:

```
thor wallets use Dariusz
```

### 3. Retrieving wallet state:


```
thor wallets status
```
NOTICE:  lack of --alias parameter as we set default wallet alias before

### 4. Retrieving wallet state:


```
thor wallets status
```

notice lack of --alias parameter as we set default wallet alias before

### 5. Sending funds to address:

a) Without waiting until fragment is put in block
```
thor send tx --ada 1 --pin 1234 --address ca1qkugtcvw43gxpl5h4w5l26hqzkhdw74caq520nt4sz54rghxg34pqnmalyc
```

b) With waiting until fragment is put in block

```
thor send --wait tx --ada 1 --pin 1234 --address ca1qkugtcvw43gxpl5h4w5l26hqzkhdw74caq520nt4sz54rghxg34pqnmalyc
```
