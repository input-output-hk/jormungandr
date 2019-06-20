+++
title = "How to install from source?"
author = ["alejandro garcia"]
draft = false
+++

Follow the instructions below or watch these video tutorials:

-   [Installing Jormungandr in linux](https://youtu.be/dhri33gWdgU)
-   [Install Jormungandr in Windows using WSL](https://youtu.be/315bQKSTdZA)

Here are the steps to install jormungandr  from source.
First you need to have configured [rustup](https://www.rust-lang.org/tools/install).

```bash
#Create a directory to store our experiments
mkdir -p ~/jor-test
cd ~/jor-test

#Get the source code
git clone https://github.com/input-output-hk/jormungandr
cd jormungandr
git submodule update --init --recursive

# Install and make the executables available in the PATH
cargo install --force --path jormungandr
cargo install --force --path jcli

# Make scripts exectuable
chmod +x ./scripts/bootstrap
```


## Verify that jcli issue installed {#verify-that-jcli-issue-installed}

Let's check if the jcli got installed.

```bash
jcli -V
```

```text
jcli 0.2.1
```

We see the version of jcli that has been installed, so far so good.
