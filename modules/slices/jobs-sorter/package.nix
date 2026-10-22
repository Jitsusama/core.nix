# The sorter that moves a terminal's busy work into the jobs slice, built
# from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "jobs-sorter";
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
    description = "Moves a terminal's busy work into the jobs slice";
    license = lib.licenses.mit;
    mainProgram = "jobs-sorter";
  };
}
