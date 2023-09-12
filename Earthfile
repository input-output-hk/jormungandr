# Set the Earthly version to 0.7
VERSION 0.7
FROM debian:stable-slim

rust-toolchain:
    FROM rust:1.70-slim-bullseye
    RUN rustup component add rustfmt

# Installs Cargo chef
install-chef:
    FROM +rust-toolchain
    RUN cargo install --debug cargo-chef

# Prepares the local cache
prepare-cache:
    FROM +install-chef
    COPY --dir jormungandr jormungandr-lib jcli explorer modules testing .
    COPY --dir Cargo.lock Cargo.toml .
    RUN cargo chef prepare
    SAVE ARTIFACT recipe.json
    SAVE IMAGE --cache-hint

# Builds the local cache
build-cache:
    FROM +install-chef
    COPY +prepare-cache/recipe.json ./

    # Install build dependencies
    RUN apt-get update && \
        apt-get install -y --no-install-recommends \
        build-essential \
        libssl-dev \
        libpq-dev \
        libsqlite3-dev \
        pkg-config \
        protobuf-compiler

    RUN cargo chef cook --release
    SAVE ARTIFACT target
    SAVE ARTIFACT $CARGO_HOME cargo_home
    SAVE IMAGE --cache-hint

# This is the default builder that all other builders should inherit from
builder:
    FROM +rust-toolchain

    WORKDIR /src

    # Install build dependencies
    RUN apt-get update && \
        apt-get install -y --no-install-recommends \
        build-essential \
        libssl-dev \
        libpq-dev \
        libsqlite3-dev \
        pkg-config \
        protobuf-compiler
    COPY --dir jormungandr jormungandr-lib jcli explorer modules testing .
    COPY --dir Cargo.lock Cargo.toml .
    COPY +build-cache/cargo_home $CARGO_HOME
    COPY +build-cache/target target
    SAVE ARTIFACT /src

build:
    FROM +builder

    COPY --dir jormungandr jormungandr-lib jcli explorer modules testing .
    COPY Cargo.toml Cargo.lock ./

    RUN cargo build --locked --release -p jormungandr -p jcli -p explorer

    SAVE ARTIFACT /src/target/release/explorer explorer
    SAVE ARTIFACT /src/target/release/jcli jcli
    SAVE ARTIFACT /src/target/release/jormungandr jormungandr
    SAVE IMAGE --cache-hint

docker:
    FROM debian:stable-slim

    WORKDIR /app
    ARG tag="latest"
    ARG registry

    # Install voting-node system dependencies
    RUN apt-get update && \
        apt-get install -y --no-install-recommends \
        libpq5 \
        openssh-client \
        build-essential \
        libxml2-dev \
        libxslt-dev \
        zlib1g-dev

    ## apt cleanup
    RUN apt-get clean && \
        rm -rf /var/lib/apt/lists/*

    COPY +build/jormungandr .
    COPY +build/jcli .
    COPY entry.sh .
    RUN chmod +x entry.sh

    ENV BIN_PATH=/app/jormungandr
    ENTRYPOINT ["/app/entry.sh"]
    SAVE IMAGE --push ${registry}jormungandr:$tag
