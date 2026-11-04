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
        packages = rec {
          daemon = rust.craneLib.buildPackage {
            pname = "flow-nexus-daemon"; version = "1.0.0"; src = source; inherit cargoArtifacts;
            cargoExtraArgs = "-p flow-nexus-daemon --bin flow-nexus-daemon";
          };
          translator = rust.craneLib.buildPackage {
            pname = "flow-nexus-translator"; version = "1.0.0"; src = source; inherit cargoArtifacts;
            cargoExtraArgs = "-p flow-nexus-translator --bin flow-nexus-translator";
          };
          engines = rust.craneLib.buildPackage {
            pname = "flow-nexus-engines"; version = "1.0.0"; src = source; inherit cargoArtifacts;
            cargoExtraArgs = "-p flow-nexus-translator --bin flow-pair-engine --bin flow-nexus-engine";
            postInstall = ''
              mkdir -p "$out/libexec"
              mv "$out/bin/flow-pair-engine" "$out/libexec/"
              mv "$out/bin/flow-nexus-engine" "$out/libexec/"
            '';
          };
          pair = pkgs.writeShellScriptBin "flow-pair" ''
            if [[ $# -ne 1 ]]; then echo "usage: flow-pair '<Place>'" >&2; exit 64; fi
            exec ${engines}/libexec/flow-pair-engine "$1" ${daemon}/bin/flow-nexus-daemon ${translator}/bin/flow-nexus-translator 70000 8388608 25 1048576 64
          '';
          cli = pkgs.writeShellScriptBin "flow-nexus" ''
            if [[ $# -ne 2 ]]; then echo "usage: flow-nexus METAFLOW DATOM" >&2; exit 64; fi
            exec ${engines}/libexec/flow-nexus-engine "$1" "$2" 70000 8388608 25
          '';
          default = pkgs.symlinkJoin { name = "flow-nexus-v1"; paths = [ daemon translator pair cli ]; };
        };
        checks = rec {
          test = rust.craneLib.cargoTest { src = source; inherit cargoArtifacts; cargoTestExtraArgs = "--workspace --all-features"; };
          operation-generated = generatedCheck "operation" ./ethos/operation.ethos ./crates/core/src/generated/operation.rs;
          memory-generated = generatedCheck "memory" ./ethos/memory.ethos ./crates/core/src/generated/memory.rs;
          emission-generated = generatedCheck "emission" ./ethos/emission.ethos ./crates/emission/src/generated/emission.rs;
          generated = pkgs.runCommand "flow-generated-contracts" { contracts = [ operation-generated memory-generated emission-generated ]; } ''
            for contract in $contracts; do test -e "$contract"; done
            touch "$out"
          '';
        };
      });
}
