{
  inputs = {
    utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, utils }:
  let
    overlay = self: super: {
      jormungandr = self.callPackage ./jormungandr.nix {};
    };
  in
    (utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (system:
      let
        pkgs = import nixpkgs { inherit system; overlays = [ overlay ]; };
      in {
        packages.jormungandr = pkgs.jormungandr;
        defaultPackage = pkgs.jormungandr;
      }
    )) // {
      inherit overlay;
    };
}
