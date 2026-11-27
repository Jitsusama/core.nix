# subject-hubs, built from this directory's crate, a library with no
# program, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "subject-hubs";
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
    description = "Reads the subjects' hubs a mycelium node names and keeps the needs put to them";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}
