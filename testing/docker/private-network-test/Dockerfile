# Simple dockerfile example to build a jormungandr and start in genesis mode

FROM ubuntu:18.04
LABEL MAINTAINER IOHK
LABEL description="Jormungandr scenario tests"

ARG PREFIX=/app
ENV ENV_PREFIX="testnet_scripts"

# prepare the environment
RUN apt-get update && \
    apt-get install -y git curl && \
    mkdir -p ${ENV_PREFIX} && \
    cd ${ENV_PREFIX} && \
    git clone --recurse-submodules https://github.com/input-output-hk/jormungandr src
    
#install rustup
RUN  apt-get install -y build-essential pkg-config libssl-dev && \
    bash -c "curl https://sh.rustup.rs -sSf | bash -s -- -y" && \
     ~/.cargo/bin/rustup install stable && \
    ~/.cargo/bin/rustup default stable


# install the node and jcli from source
RUN cd ${ENV_PREFIX}/src && \
    git submodule update --init --recursive && \
    ~/.cargo/bin/cargo build --release && \
    ~/.cargo/bin/cargo install --locked --force --path jormungandr && \
    ~/.cargo/bin/cargo install --locked --force --path jcli  && \
    ~/.cargo/bin/cargo install --locked --force --path testing/jormungandr-scenario-tests 

WORKDIR ${ENV_PREFIX}/src/testing/jormungandr-scenario-tests
RUN ["/root/.cargo/bin/jormungandr-scenario-tests", "--jcli", "/root/.cargo/bin/jcli","--jormungandr", "/root/.cargo/bin/jormungandr","--tag","short","--progress-bar-mode", "standard", "--report-unstable","--print-report"]