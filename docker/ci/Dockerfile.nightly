FROM rustlang/rust:nightly

LABEL org.label-schema.schema-version = "1.0"
LABEL org.label-schema.vendor = "Input Output HK Limited"
LABEL org.label-schema.description = "CI image for Jormungandr (nightly Rust)"

RUN apt-get update && \
    apt-get install -y protobuf-compiler && \
    apt-get clean && \
    cargo install cargo-audit && \
    rm -rf $HOME/.cargo/git && \
    rm -rf $HOME/.cargo/registry
