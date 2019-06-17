+++
title = "How to start a self-node?"
author = ["alejandro garcia"]
draft = false
+++

## Execute bootstrap script {#execute-bootstrap-script}

We can use the bootsrap script to send a single node.

-   We create directory `jor-test/self_node` to store all our configurations

```bash
mkdir -p ~/jor-test/self_node
cd ~/jor-test/self_node

```

-   Now let's execute the bootstrap script. We use `tee` command since we want to display and save the output to file.

<a id="code-snippet--initial-configuration"></a>
```bash
~/jor-test/jormungandr/scripts/bootstrap | tee initial_configuration.txt
```

```text
########################################################

* Consensus: genesis
* REST Port: 8443
* Slot duration: 10
#######################################################

* CLI version: jcli 0.2.1
* NODE version: jormungandr 0.2.1
#######################################################

faucet account: ta1s5wzff5jvlatyygnadp9c6zkpznap4ngkd5zed4gxrh5ela3kjrlw4nmz3l
  * public: ed25519_pk1rsj2dyn8l2epzyltgfwxs4sg5lgdv69ndqktd2psaax0lvd5slmsaf5a05
  * secret: ed25519e_sk1uz3x6jkvptx2xv9rpymrvqerk725ahpq29wwcws58vash78aza0knr6cw8zh3k3qrxh8gn2xufztpqcnydrejk6dlj8yw3tv4286g9qn69k3r
  * amount: 1000000000
pool id: cc77d20db4967c554ca1593ddd40f8ffd09e41aac2e9708b49f9f1237fbcb209
To start the node:
  jormungandr --genesis-block ./block-0.bin --config ./config.yaml --secret ./pool-secret1.yaml
To connect using CLI REST:
--host "http://127.0.0.1:8443/api"
For example:
  jcli rest v0 node stats -h "http://127.0.0.1:8443/api"
```


## Review the files that bootstrap created {#review-the-files-that-bootstrap-created}

The files that bootstrap created are:

```text
ls -l

total 220
-rw-r--r-- 1 agarciafdz agarciafdz    544 jun 15 21:05 block-0.bin
-rw-r--r-- 1 agarciafdz agarciafdz    261 jun 15 21:05 config.yaml
-rwxr-xr-x 1 agarciafdz agarciafdz   2801 jun 16 19:41 faucet-send-certificate.sh
-rwxr-xr-x 1 agarciafdz agarciafdz   2990 jun 15 21:05 faucet-send-money.sh
-rw-r--r-- 1 agarciafdz agarciafdz   1062 jun 15 21:05 genesis.yaml
-rw-r--r-- 1 agarciafdz agarciafdz    955 jun 15 21:05 initial_configuration.txt
drwxr-xr-x 2 agarciafdz agarciafdz   4096 jun 16 07:58 jormungandr-storage-test
-rw-r--r-- 1 agarciafdz agarciafdz   2237 jun 15 21:05 pool-secret1.yaml
```

we have:

block-0.bin
: That is the hash version of the first block

config.yaml
: that has the configuration options of the node

faucet-send-certificate.sh
: this script was created for us and will be used when we need to delegate.

faucet-send-money.sh
: another script created for us that sends money from the faucet to another account.

initial\_configuration.txt:
: We created this with the tee command. just to save secret keys

pool-secret1.yaml
: configuration file of the stakepool in this case we only have 1 stakepool.


## Start the node {#start-the-node}

The bootstra script also suggests the command to run a node.
In this example we are going to send the log messages to a file with `&>my_node.log`, so that we can analyze them later.
An we will run the process in the background with `&`.

```bash
jormungandr --genesis-block ./block-0.bin --config ./config.yaml --secret ./pool-secret1.yaml &> my_node.log &
```

you can check what the log has with the tail command

```bash
tail my_node.log
```

```text
src/src_libsinglebin_mkfifo_a-mkfifo.o
Jun 15 21:21:46.613 INFO storing blockchain in '"/home/agarciafdz/jor-test/self_node/jormungandr-storage-test/blocks.sqlite"', task: init
Jun 15 21:21:46.615 WARN no gRPC peers specified, skipping bootstrap, task: bootstrap
Jun 15 21:21:46.615 INFO starting task, task: client-query
Jun 15 21:21:46.615 INFO starting task, task: network
Jun 15 21:21:46.615 INFO our node id: 237942958938056621784374166035673708836, task: network
Jun 15 21:21:46.615 INFO adding P2P Topology module: trusted-peers, task: network
Jun 15 21:21:46.615 INFO start listening and accepting gRPC connections on 127.0.0.1:8299, task: network
Jun 15 21:21:46.616 INFO preparing, task: leadership
Jun 15 21:21:46.616 INFO starting, task: leadership
Jun 15 21:21:46.616 INFO starting, sub_task: End Of Epoch Reminder, task: leadership
```

n


## Check initial balance {#check-initial-balance}

Now that we have a node running let's check the initial balance of the faucet account.

In case you forgot the data you can check the initial configuration.txt file

```bash
cat initial_configuration.txt
```

There we can see that the initial private key is:

```text
ed25519e_sk1hqswug8fwajqyz247wn6yhaj080nkd6thtxcv0cshgmhpezj2aqvctwspne3q52fl29tag3nhejezu83tuqvfdtknre3hkhmya48a5cuahdj4
```

And the faucet account is:

```text
ta1s5p929pwyz0kkh2gk2mpl02q5ucwwdyz5qs7qsga5jwghx72ztfkj9salpw
```

With that information we can check the original balance in the account.

```bash
jcli rest v0 account get $FAUCET_ACCOUNT -h  http://127.0.0.1:8443/api
```

```text

---
counter: 0
delegation:
  - 137
  - 100
  - 159
  - 208
  - 207
  - 115
  - 164
  - 132
  - 57
  - 164
  - 112
  - 209
  - 246
  - 212
  - 70
  - 140
  - 237
  - 137
  - 231
  - 121
  - 109
  - 66
  - 226
  - 115
  - 32
  - 13
  - 84
  - 161
  - 74
  - 64
  - 126
  - 254
value: 1000000000
```

It should be the same that we created with bootstrap, 1000000000 tokens.
