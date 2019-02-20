{ nixpkgs ? fetchTarball channel:nixos-unstable
, pkgs ? import nixpkgs {}
}:

with pkgs;

stdenv.mkDerivation {
  name = "jormungandr";

  src = null;

  buildInputs = [ rustup rustc cargo sqlite protobuf rustfmt pkgconfig openssl ];

  # FIXME: we can remove this once prost is updated.
  PROTOC = "${protobuf}/bin/protoc";
}
