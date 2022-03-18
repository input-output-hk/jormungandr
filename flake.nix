{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  #inputs.naersk.url = "github:nix-community/naersk";
  # XXX: https://github.com/nix-community/naersk/pull/167
  inputs.naersk.url = "github:yusdacra/naersk/feat/cargolock-git-deps";
  inputs.naersk.inputs.nixpkgs.follows = "nixpkgs";
  # TODO: use pre-commit-hooks

  nixConfig.extra-substituters =
    [ "https://hydra.iohk.io"
      "https://vit-ops.cachix.org"
    ];
  nixConfig.extra-trusted-public-keys =
    [ "hydra.iohk.io:f/Ea+s+dFdN+3Y/G+FDgSq+a5NEWhJGzdjvKNGv0/EQ="
      "vit-ops.cachix.org-1:LY84nIKdW7g1cvhJ6LsupHmGtGcKAlUXo+l1KByoDho="
    ];

  outputs = { self
            , nixpkgs
            , flake-utils
            , rust-overlay
            , naersk
            }:
    flake-utils.lib.eachSystem
      [ flake-utils.lib.system.x86_64-linux
        flake-utils.lib.system.aarch64-linux
      ]
      (system: 
        let
          readTOML = file: builtins.fromTOML (builtins.readFile file);
          workspaceCargo = readTOML ./Cargo.toml;

          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };

          rust = pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analysis"
              "rustfmt-preview"
              "clippy-preview"
            ];
          };

          naersk-lib = naersk.lib."${system}".override {
            cargo = rust;
            rustc = rust;
          };

          mkPackage = name:
            let pkgCargo = readTOML ./${name}/Cargo.toml;
            in
              naersk-lib.buildPackage {
                #src = self + "/${name}";
                root = ./.;

                cargoBuildOptions = x: x ++ [ "-p" name ];
                cargoTestOptions = x: x ++ [ "-p" name ];

                PROTOC = "${pkgs.protobuf}/bin/protoc";
                PROTOC_INCLUDE = "${pkgs.protobuf}/include";

                nativeBuildInputs = with pkgs; [
                  pkg-config
                  protobuf
                  rustfmt
                ];

                buildInputs = with pkgs; [
                  openssl
                ];
              };

          workspace =
            builtins.listToAttrs
              (builtins.map
                (name: { inherit name; value = mkPackage name; })
                workspaceCargo.workspace.members
              );

          jormungandr-entrypoint =
            let
              script = pkgs.writeShellScriptBin "jormungandr-entrypoint"
              ''
                set -exuo pipefail

                ulimit -n 1024

                nodeConfig="$NOMAD_TASK_DIR/node-config.json"
                runConfig="$NOMAD_TASK_DIR/running.json"
                runYaml="$NOMAD_TASK_DIR/running.yaml"
                name="jormungandr"

                chmod u+rwx -R "$NOMAD_TASK_DIR" || true

                function convert () {
                  chmod u+rwx -R "$NOMAD_TASK_DIR" || true
                  cp "$nodeConfig" "$runConfig"
                  remarshal --if json --of yaml "$runConfig" > "$runYaml"
                }

                if [ "$RESET" = "true" ]; then
                  echo "RESET is given, will start from scratch..."
                  rm -rf "$STORAGE_DIR"
                elif [ -d "$STORAGE_DIR" ]; then
                  echo "$STORAGE_DIR found, not restoring from backup..."
                else
                  echo "$STORAGE_DIR not found, restoring backup..."

                  restic restore latest \
                    --verbose=5 \
                    --no-lock \
                    --tag "$NAMESPACE" \
                    --target / \
                  || echo "couldn't restore backup, continue startup procedure..."
                fi

                set +x
                echo "waiting for $REQUIRED_PEER_COUNT peers"
                until [ "$(jq -e -r '.p2p.trusted_peers | length' < "$nodeConfig" || echo 0)" -ge $REQUIRED_PEER_COUNT ]; do
                  sleep 1
                done
                set -x

                convert

                if [ -n "$PRIVATE" ]; then
                  echo "Running with node with secrets..."
                  exec jormungandr \
                    --storage "$STORAGE_DIR" \
                    --config "$NOMAD_TASK_DIR/running.yaml" \
                    --genesis-block $NOMAD_TASK_DIR/block0.bin/block0.bin \
                    --secret $NOMAD_SECRETS_DIR/bft-secret.yaml \
                    "$@" || true
                else
                  echo "Running with follower node..."
                  exec jormungandr \
                    --storage "$STORAGE_DIR" \
                    --config "$NOMAD_TASK_DIR/running.yaml" \
                    --genesis-block $NOMAD_TASK_DIR/block0.bin/block0.bin \
                    "$@" || true
                fi
              '';
            in pkgs.symlinkJoin {
              name = "entrypoint";
              paths = [ script workspace.jormungandr ] ++ (with pkgs; [
                bashInteractive
                coreutils
                curl
                diffutils
                fd
                findutils
                gnugrep
                gnused
                htop
                jq
                lsof
                netcat
                procps
                remarshal
                restic
                ripgrep
                strace
                tcpdump
                tmux
                tree
                utillinux
                vim
                yq
              ]);
            };

        in rec {
          packages =
            { inherit (workspace) jormungandr jcli;
              inherit jormungandr-entrypoint;
            };

          devShell = pkgs.mkShell {
            PROTOC = "${pkgs.protobuf}/bin/protoc";
            PROTOC_INCLUDE = "${pkgs.protobuf}/include";
            buildInputs = [ rust ] ++ (with pkgs; [
              pkg-config
              openssl
              protobuf
            ]);
          };

          hydraJobs = packages;

          # TODO: ciceroActions
        }
      );
}
