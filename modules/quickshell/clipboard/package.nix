# The clipboard's history for the launcher, built from this directory, its
# tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "clipboard-history";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Keeps the clipboard's copies, text and pictures, for the launcher";
    license = lib.licenses.mit;
    mainProgram = "clipboard-history";
  };
}
