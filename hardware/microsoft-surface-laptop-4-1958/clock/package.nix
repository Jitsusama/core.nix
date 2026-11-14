# The program that keeps the Surface Aggregator's clock at the true time,
# built from this directory, its tests run as it builds. It takes its shape
# from the terminal kit (modules/terminal-kit), so the source is cut from
# the repository's root, which holds both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "surface-clock";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../../../modules/terminal-kit/Cargo.toml
      ../../../modules/terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/hardware/microsoft-surface-laptop-4-1958/clock";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Keeps the Surface Aggregator's clock, which the firmware starts from, at the true time";
    license = lib.licenses.mit;
    mainProgram = "surface-clock";
  };
}
