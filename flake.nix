{
  description = "Flow Nexus version one";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-build = { url = "github:LiGoldragon/rust-build"; inputs.nixpkgs.follows = "nixpkgs"; };
    ethos-zero.url = "github:LiGoldragon/ethos-zero/b8052c2404374265dfcddbde83fef76977f78acd";
  };
  outputs = { nixpkgs, flake-utils, rust-build, ethos-zero, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        rust = rust-build.lib.${system}.fromPkgs pkgs;
        source = rust.cleanSource {
          root = ./.;
          extraFilters = [ (path: type: type == "regular" && pkgs.lib.hasSuffix ".ethos" (toString path)) ];
        };
        cargoArtifacts = rust.craneLib.buildDepsOnly { src = source; cargoExtraArgs = "--workspace --all-features"; };
        generatedCheck = name: declaration: committed:
          pkgs.runCommand "flow-${name}-generated" { generator = ethos-zero.packages.${system}.default; inherit declaration committed; }
            (builtins.readFile ./checks/generated-contract.sh);
      in {
        checks = rec {
          test = rust.craneLib.cargoTest { src = source; inherit cargoArtifacts; cargoTestExtraArgs = "--workspace --all-features"; };
          operation-generated = generatedCheck "operation" ./ethos/operation.ethos ./crates/core/src/generated/operation.rs;
          emission-generated = generatedCheck "emission" ./ethos/emission.ethos ./crates/emission/src/generated/emission.rs;
          generated = pkgs.runCommand "flow-generated-contracts" { contracts = [ operation-generated emission-generated ]; } ''
            for contract in $contracts; do test -e "$contract"; done
            touch "$out"
          '';
        };
      });
}
