# The program that tells Plymouth when a shutdown is forced on or held up,
# built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "shutdown-status";
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
    description = "Tells Plymouth when a shutdown is forced on or held up";
    license = lib.licenses.mit;
    mainProgram = "shutdown-status";
  };
}
