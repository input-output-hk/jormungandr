# Set the Earthly version to 0.7
VERSION 0.7

rust-toolchain:
    FROM rust:1.71-slim-bullseye
    RUN rustup component add rustfmt

# Installs Cargo chef
install-chef:
    FROM +rust-toolchain
    RUN cargo install --debug cargo-chef

# Prepares the local cache
prepare-cache:
    FROM +install-chef
    COPY --dir jormungandr jcli jormungandr-lib explorer modules testing .
    COPY Cargo.lock Cargo.toml .
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
    COPY --dir jormungandr jcli .
    COPY Cargo.lock Cargo.toml .
    COPY +build-cache/cargo_home $CARGO_HOME
    COPY +build-cache/target target
    SAVE ARTIFACT /src

build:
    FROM +builder

    COPY --dir jormungandr jcli jormungandr-lib explorer modules testing .
    COPY Cargo.lock Cargo.toml .

    RUN cargo build --locked --release -p jormungandr -p jcli

    SAVE ARTIFACT /src/target/release/jormungandr jormungandr
    SAVE ARTIFACT /src/target/release/jcli jcli

publish:
    FROM debian:stable-slim
    WORKDIR /app

    ARG tag=latest
    ARG fetcher_version=2.2.4

    # Install build dependencies
    RUN apt-get update && \
        apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl-dev \
        libpq-dev \
        libsqlite3-dev \
        tar \
        zstd

    # Install fetcher
    IMPORT github.com/input-output-hk/catalyst-ci/tools/fetcher:v${fetcher_version} AS fetcher
    COPY fetcher+build/fetcher /usr/local/bin/fetcher

    COPY +build/jormungandr .
    COPY entrypoint.sh .
    RUN chmod +x entrypoint.sh

    ENTRYPOINT ["/app/entrypoint.sh"]

    SAVE IMAGE jormungandr:${tag}