# Simple dockerfile example to build a jormungandr and start in genesis mode

FROM ubuntu:cosmic
LABEL MAINTAINER IOHK
LABEL description="Jormungandr latest"

ARG PREFIX=/app
ARG REST_PORT=8448
ARG GRPC_PORT=8299

ENV ENV_PREFIX=${PREFIX}

# prepare the environment
RUN apt-get update && \
    apt-get install -y build-essential pkg-config git libssl-dev curl && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# install the node and jcli from source
RUN mkdir -p ${ENV_PREFIX} && \
    mkdir -p ${ENV_PREFIX}/src && \
    mkdir -p ${ENV_PREFIX}/bin && \
    bash -c "curl https://sh.rustup.rs -sSf | bash -s -- -y" && \
    export PATH=$HOME/.cargo/bin:$PATH && \
    rustup install stable && \
    rustup default stable && \
    cd ${ENV_PREFIX} && \
    git clone --recurse-submodules https://github.com/input-output-hk/jormungandr src && \
    cd src && \
    cargo build --release && \
    cargo install --force --path jormungandr && \
    cargo install --force --path jcli && \
    cp scripts/bootstrap \
        scripts/faucet-send-money.shtempl scripts/faucet-send-certificate.shtempl scripts/create-account-and-delegate.shtempl scripts/jcli-helpers \
        $HOME/.cargo/bin/jormungandr $HOME/.cargo/bin/jcli ${ENV_PREFIX}/bin && \
    rm -rf $HOME/.cargo $HOME/.rustup ${ENV_PREFIX}/src

ENV PATH=${ENV_PREFIX}/bin:${PATH}
WORKDIR ${ENV_PREFIX}/bin

RUN mkdir -p ${ENV_PREFIX}/cfg && \
    mkdir -p ${ENV_PREFIX}/secret && \
    sh ./bootstrap -p ${REST_PORT} -x -c ${ENV_PREFIX}/cfg -k ${ENV_PREFIX}/secret

# expose rest API port
EXPOSE ${REST_PORT}

# expose gRPC port
EXPOSE ${GRPC_PORT} 

CMD [ "sh", "startup_script.sh" ]
