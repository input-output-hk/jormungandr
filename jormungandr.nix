{ lib, rustPlatform, fetchFromGitHub, pkg-config, openssl, protobuf, rustfmt }:
let
  cargoPackage = (builtins.fromTOML (builtins.readFile ./jormungandr/Cargo.toml)).package;
in
rustPlatform.buildRustPackage rec {
  pname = cargoPackage.name;
  version = cargoPackage.version;
  src = ./.;
  cargoSha256 = "sha256-rfgmtKweYxk8BELv2YoF3BsGfdGlQGP8HNLE4xklDHs=";
  nativeBuildInputs = [ pkg-config protobuf rustfmt ];
  buildInputs = [ openssl ];
  configurePhase =''
    cc=$CC
  '';
  doCheck = false;
  doInstallCheck = false;
  PROTOC="${protobuf}/bin/protoc";
  PROTOC_INCLUDE="${protobuf}/include";
}
