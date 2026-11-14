# The program that has logind's lock against the lid's handling held while a
# monitor is plugged in, built from this directory, its tests run as it
# builds. It takes its shape from the terminal kit (../../terminal-kit), so
# the source is cut from the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "lid-docked";
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
  sourceRoot = "${src.name}/lid/docked";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Has logind's lock against the lid held while a monitor is plugged in";
    license = lib.licenses.mit;
    mainProgram = "lid-docked";
  };
}
