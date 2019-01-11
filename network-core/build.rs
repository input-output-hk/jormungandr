fn main() {
    prost_build::compile_protos(&["../network-proto/codes.proto"], &["../network-proto"]).unwrap();
}
