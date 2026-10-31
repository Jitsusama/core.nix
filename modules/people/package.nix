# The people daemon, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "people";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Holds everyone Joel deals with ready for the shell, from each People source";
    license = lib.licenses.mit;
    mainProgram = "people";
    platforms = lib.platforms.linux;
  };
}
