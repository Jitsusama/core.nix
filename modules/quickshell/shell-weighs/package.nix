# What the shell weighs, built from this directory, its tests run as it
# builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "shell-weighs";
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
    description = "Says what the shell weighs, part by part, read from outside it";
    license = lib.licenses.mit;
    mainProgram = "shell-weighs";
    platforms = lib.platforms.linux;
  };
}
