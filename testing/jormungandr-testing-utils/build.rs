fn main() {
    tonic_build::compile_protos("../../chain-deps/chain-network/proto/node.proto").unwrap();
}
