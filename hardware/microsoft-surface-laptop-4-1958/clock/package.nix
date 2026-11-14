# The program that keeps the Surface Aggregator's clock at the true time,
# built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "surface-clock";
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
    description = "Keeps the Surface Aggregator's clock, which the firmware starts from, at the true time";
    license = lib.licenses.mit;
    mainProgram = "surface-clock";
  };
}
