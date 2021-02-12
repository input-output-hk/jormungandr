{
  inputs = {
    utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, utils }:
  utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (system:
  let
    cargoPackage =
      (builtins.fromTOML (builtins.readFile ./jormungandr/Cargo.toml)).package;

    overlay = self: super: {
      jormungandr = self.callPackage (
        { lib, rustPlatform, fetchFromGitHub, pkg-config, openssl, protobuf, rustfmt }:
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
      ) {};
    };
    pkgs = import nixpkgs { inherit system; overlays = [ overlay ]; };
  in {
    packages.jormungandr = pkgs.jormungandr;
    defaultPackage = pkgs.jormungandr;
    inherit overlay;
  });
}
