# Build a wire crate with its explicitly supplied sibling type-library source.
# Both inputs must be clean, content-addressed Nix sources.
{ pkgs, craneLib, flowSource, wireSource, name }:
let
  src = pkgs.runCommand "${name}-anatomy-source" { } ''
    mkdir -p "$out"
    cp -r ${flowSource} "$out/flow-nexus"
    cp -r ${wireSource} "$out/${name}"
  '';
in craneLib.cargoBuild {
  inherit src;
  pname = "${name}-anatomy";
  version = "1.0.0";
  postUnpack = ''sourceRoot="$sourceRoot/${name}"'';
  cargoVendorDir = craneLib.vendorCargoDeps { cargoLock = wireSource + "/Cargo.lock"; };
  cargoArtifacts = null;
  doInstallCargoArtifacts = false;
  cargoExtraArgs = "--locked --all-features";
  strictDeps = true;
  doCheck = false;
}
