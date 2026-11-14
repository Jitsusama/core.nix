# What starts pi's shell commands for it, built from this folder, its
# tests run as it builds (tests/spawner.rs). The agent harness starts it
# once for each pi and falls back to pi's own spawn without it. It takes its
# help from the terminal kit (../../terminal-kit), so the source is cut from
# the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "pi-spawner";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../../terminal-kit/Cargo.toml
      ../../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/pi/spawner";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Starts pi's shell commands for it, from a process small enough to start them cheaply";
    license = lib.licenses.mit;
    mainProgram = "pi-spawner";
    platforms = lib.platforms.unix;
  };
}
