# The program that has logind's lock against the lid's handling held while a
# monitor is plugged in, built from this directory, its tests run as it
# builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "lid-docked";
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
    description = "Has logind's lock against the lid held while a monitor is plugged in";
    license = lib.licenses.mit;
    mainProgram = "lid-docked";
  };
}
