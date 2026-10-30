{
  description = "Fresh Flow Nexus v1 anatomy; not a running service";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-build = {
      url = "github:LiGoldragon/rust-build/1bcdafd4590952f73fada56b0507c64192fd6327";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = { self, nixpkgs, rust-build }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      rust = rust-build.lib.${system}.fromPkgs pkgs;
      src = rust.cleanSource {
        root = ./.;
        extraFilters = [ (path: type: type == "regular" && pkgs.lib.hasSuffix ".ethos" path) ];
      };
      common = { inherit src; strictDeps = true; doCheck = false; };
    in {
      packages.${system}.default = rust.craneLib.cargoBuild common;
      checks.${system}.datom = rust.craneLib.cargoBuild (common // { cargoExtraArgs = "--locked --all-features"; });
      devShells.${system}.default = pkgs.mkShell { packages = [ rust.toolchain ]; };
    };
}
