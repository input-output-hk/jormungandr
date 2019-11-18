FROM circleci/rust:latest

LABEL org.label-schema.schema-version = "1.0"
LABEL org.label-schema.vendor = "Input Output HK Limited"
LABEL org.label-schema.description = "CI image for Jormungandr"

RUN sudo apt-get update && \
    sudo apt-get install -y protobuf-compiler && \
    sudo apt-get clean && \
    cargo install cargo-audit && \
    rm -rf $HOME/.cargo/git && \
    rm -rf $HOME/.cargo/registry
