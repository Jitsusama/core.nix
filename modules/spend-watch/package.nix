# spend-watch, built from this directory, its tests run as it builds. Its
# rows take the form every daemon's do (../ready-rows), so the source is
# cut from the modules folder holding both, and built from this one.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "spend-watch";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      # The feed contract's examples, which its tests read.
      ./contract
      ../ready-rows/Cargo.toml
      ../ready-rows/src
    ];
  };
  sourceRoot = "${src.name}/spend-watch";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Keeps an account told what its AI use costs";
    license = lib.licenses.mit;
    mainProgram = "spend-watch";
    platforms = lib.platforms.linux;
  };
}
