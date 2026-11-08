# `launcher`, the launcher in a terminal, built from this directory, its
# tests run as it builds. What it asks, how it inks and its words are given
# where it's installed (../home.nix), so this builds the same whatever the
# theme.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "launcher";
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
    description = "Prints any place, row or drawer the launcher holds, as text or JSON";
    license = lib.licenses.mit;
    mainProgram = "launcher";
    platforms = lib.platforms.linux;
  };
}
