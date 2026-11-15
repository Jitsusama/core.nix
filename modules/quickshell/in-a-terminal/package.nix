# `launcher`, the launcher in a terminal, built from this directory, its
# tests run as it builds. What it asks and its words are given where it's
# installed (../home.nix), and it reads the theme as it runs, so this builds
# the same whatever the theme. Its source reaches up to modules/ for the terminal kit, the
# shape every command of ours takes, and builds from its own folder there.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "launcher";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../../terminal-kit/Cargo.toml
      ../../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/quickshell/in-a-terminal";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Prints any place, row or drawer the launcher holds, as text or JSON";
    license = lib.licenses.mit;
    mainProgram = "launcher";
    platforms = lib.platforms.linux;
  };
}
