FROM ubuntu:18.04
LABEL MAINTAINER IOHK
LABEL description="Jormungandr non functional tests"

ARG PREFIX=/app
ENV ENV_PREFIX="non_functional_tests"

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
    ~/.cargo/bin/cargo install --locked --force --path jormungandr && \
    ~/.cargo/bin/cargo install --locked --force --path jcli
    
# run tests
WORKDIR ${ENV_PREFIX}/src/testing/jormungandr-integration-tests
RUN ["bash", "-c", "~/.cargo/bin/cargo test  non_functional --features sanity-non-functional --release -- --nocapture --test-threads=1 -Z unstable-options --format json"]