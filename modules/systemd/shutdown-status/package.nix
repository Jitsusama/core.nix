# The program that tells Plymouth when a shutdown is forced on or held up,
# built from this directory, its tests run as it builds. It takes its shape
# from the terminal kit (../../terminal-kit), so the source is cut from the
# folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "shutdown-status";
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
  sourceRoot = "${src.name}/systemd/shutdown-status";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Tells Plymouth when a shutdown is forced on or held up";
    license = lib.licenses.mit;
    mainProgram = "shutdown-status";
  };
}
