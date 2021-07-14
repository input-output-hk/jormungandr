FROM rust:1.53-slim
RUN apt-get update && apt-get install -y pkg-config libssl-dev && apt-get clean
RUN cargo install cargo-audit
