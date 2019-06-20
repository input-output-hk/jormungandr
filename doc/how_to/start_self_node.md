+++
title = "How to start a self node?"
author = ["alejandro garcia"]
draft = false
+++

Follow the instructions below or watch this video tutorial: [Jormungandr bootstrappig](https://youtu.be/M%5F7ZJKQnv%5FY)


## Execute the bootstrap script {#execute-the-bootstrap-script}

Use the bootstrap script to start a single self node.

Create directory `jor-test/self-node` to store the configurations

```bash
mkdir -p ~/jor-test/self-node
cd ~/jor-test/self-node
```

Now let's execute the bootstrap script. Use `tee` command since you want to display and save the output to file.

<a id="code-snippet--initial-configuration"></a>
```bash
~/jor-test/jormungandr/scripts/bootstrap | tee initial-configuration.txt
```

```bash
########################################################

* Consensus: genesis
* REST Port: 8443
* Slot duration: 10

########################################################

* CLI version: jcli 0.2.1
* NODE version: jormungandr 0.2.1

########################################################

faucet account: ta1shd54nk2jff4cqgqwcrwu99r4fsmk5a00k02f6cdrk7pykpaqd0mkecjp7l
  * public: ed25519_pk1md9vaj5j2dwqzqrkqmhpfga2vxa48tman6jwkrgahsf9s0grt7aszahkas
  * secret: ed25519e_sk1nzpafhpaqnfhzqw3ll87z58760ypq9fgft2lfdmfqg0fe8hjtdwytsk9gh5jjldnqw0mg0c7cns7lsddw5l5p8uhy3fjtfujvscs0mgf4wah9
  * amount: 1000000000

pool id: 2487fa8185be0397126298be7622c44454f8747aa8bb82f94827191b437ed5b3
block-0 hash: aa7971ddc287b70643622213fd54b0b6ce40dd9a60255531b3c5a2b7e3c93ac4

To start the node:
  jormungandr --genesis-block ./block-0.bin --config ./config.yaml --secret ./pool-secret1.yaml
To connect using CLI REST:
  jcli rest v0 <CMD> --host "http://127.0.0.1:8443/api"
For example:
  jcli rest v0 node stats get -h "http://127.0.0.1:8443/api"
```

It's important to note that the `bootstrap` script has several parameters the most import ones are:

-p
: Setting the port for the  REST api. By default it's 8443

-b
: Start an Ouroboros **BFT** blockchain. BFT is a good blockchain to use for debugging purposes, since slots are warranted to last 10 seconds.

-g
: Start an Ouroboros **Genesis** blockchain. It's the newest protocol. Slot duration is variable.

To check the other parameters just do a

```text
bootstrap -h
```


## Review the files that were created by the bootstrap script {#review-the-files-that-were-created-by-the-bootstrap-script}

The files that the bootstrap script created are:

```text
ls -l

total 220
-rw-r--r-- 1 agarciafdz agarciafdz    544 jun 15 21:05 block-0.bin
-rw-r--r-- 1 agarciafdz agarciafdz    261 jun 15 21:05 config.yaml
-rwxr-xr-x 1 agarciafdz agarciafdz   2801 jun 16 19:41 faucet-send-certificate.sh
-rwxr-xr-x 1 agarciafdz agarciafdz   2990 jun 15 21:05 faucet-send-money.sh
-rw-r--r-- 1 agarciafdz agarciafdz   1062 jun 15 21:05 genesis.yaml
-rw-r--r-- 1 agarciafdz agarciafdz    955 jun 15 21:05 initial-configuration.txt
drwxr-xr-x 2 agarciafdz agarciafdz   4096 jun 16 07:58 jormungandr-storage-test
-rw-r--r-- 1 agarciafdz agarciafdz   2237 jun 15 21:05 pool-secret1.yaml
```

These files include:

block-0.bin
: The encoded version of the `genesis.yaml` file

config.yaml
: Contains the configuration options of the **node**. Not to be confused with the configuration of the blockchain (in genesis.yaml)

faucet-send-certificate.sh
: Script created by bootstrap and will be used when we need to delegate stake

faucet-send-money.sh
: Another script created by bootstrap. It sends money from the faucet to another account.

genesis.yaml
: Configuration options of the **blockchain**.

initial-configuration.txt:
: This was created by the `tee` command to save relevant keys used later.

pool-secret1.yaml
: The configuration file of the stakepool (in this case we only have one stakepool).


## Starting the self node {#starting-the-self-node}

The bootstrap script also suggests the command to run a node.
In this example, we are going to send the log messages to a file with `&>my_node.log`, so that we can analyze them later.
We will run the process in the background with `&`.

```bash
jormungandr --genesis-block ./block-0.bin --config ./config.yaml --secret ./pool-secret1.yaml &> my_node.log &
```

you can check what the log contains with the `tail` command:

```bash
tail my_node.log

```

```text
Jun 18 19:57:32.976 INFO storing blockchain in '"/home/agarciafdz/jor-test/self-node/jormungandr-storage-test/blocks.sqlite"', task: init
Jun 18 19:57:33.042 WARN no gRPC peers specified, skipping bootstrap, task: bootstrap
Jun 18 19:57:33.043 INFO starting task, task: client-query
Jun 18 19:57:33.043 INFO starting task, task: network
Jun 18 19:57:33.043 INFO our node id: 230562284678097207957874779463813143417, task: network
Jun 18 19:57:33.043 INFO adding P2P Topology module: trusted-peers, task: network
Jun 18 19:57:33.044 INFO start listening and accepting gRPC connections on 127.0.0.1:8299, task: network
Jun 18 19:57:33.044 INFO preparing, task: leadership
Jun 18 19:57:33.044 INFO starting, task: leadership
Jun 18 19:57:33.045 INFO starting, sub_task: End Of Epoch Reminder, task: leadership
```


## Checking the initial balance {#checking-the-initial-balance}

Now that we have a node running let's check the initial balance of the faucet account.

In case you forgot the data, you can check the initial configuration.txt file.

```bash
cat initial-configuration.txt
```

```bash
########################################################

* Consensus: genesis
* REST Port: 8443
* Slot duration: 10

########################################################

* CLI version: jcli 0.2.1
* NODE version: jormungandr 0.2.1

########################################################

faucet account: ta1shd54nk2jff4cqgqwcrwu99r4fsmk5a00k02f6cdrk7pykpaqd0mkecjp7l
  * public: ed25519_pk1md9vaj5j2dwqzqrkqmhpfga2vxa48tman6jwkrgahsf9s0grt7aszahkas
  * secret: ed25519e_sk1nzpafhpaqnfhzqw3ll87z58760ypq9fgft2lfdmfqg0fe8hjtdwytsk9gh5jjldnqw0mg0c7cns7lsddw5l5p8uhy3fjtfujvscs0mgf4wah9
  * amount: 1000000000

pool id: 2487fa8185be0397126298be7622c44454f8747aa8bb82f94827191b437ed5b3
block-0 hash: aa7971ddc287b70643622213fd54b0b6ce40dd9a60255531b3c5a2b7e3c93ac4

To start the node:
  jormungandr --genesis-block ./block-0.bin --config ./config.yaml --secret ./pool-secret1.yaml
To connect using CLI REST:
  jcli rest v0 <CMD> --host "http://127.0.0.1:8443/api"
For example:
  jcli rest v0 node stats get -h "http://127.0.0.1:8443/api"
```

There we can see that the initial private key is:

```text
 ed25519e_sk1nzpafhpaqnfhzqw3ll87z58760ypq9fgft2lfdmfqg0fe8hjtdwytsk9gh5jjldnqw0mg0c7cns7lsddw5l5p8uhy3fjtfujvscs0mgf4wah9
```

And the faucet account is:

```text
 ta1shd54nk2jff4cqgqwcrwu99r4fsmk5a00k02f6cdrk7pykpaqd0mkecjp7l
```

With that information we can check the original balance in the account.

```bash
jcli rest v0 account get $FAUCET_ACCOUNT -h  http://127.0.0.1:8443/api
```

```text
---
counter: 0
delegation:
  - 36
  - 135
  - 250
  - 129
  - 133
  - 190
  - 3
  - 151
  - 18
  - 98
  - 152
  - 190
  - 118
  - 34
  - 196
  - 68
  - 84
  - 248
  - 116
  - 122
  - 168
  - 187
  - 130
  - 249
  - 72
  - 39
  - 25
  - 27
  - 67
  - 126
  - 213
  - 179
value: 1000000000
```

It should be the same that we created with bootstrap, 1000000000 tokens.
