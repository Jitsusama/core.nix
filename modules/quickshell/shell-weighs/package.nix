# What the shell weighs, built from this directory, its tests run as it
# builds. It takes its shape from the terminal kit (../../terminal-kit), so
# the source is cut from the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "shell-weighs";
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
  sourceRoot = "${src.name}/quickshell/shell-weighs";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says what the shell weighs, part by part, read from outside it";
    license = lib.licenses.mit;
    mainProgram = "shell-weighs";
    platforms = lib.platforms.linux;
  };
}
