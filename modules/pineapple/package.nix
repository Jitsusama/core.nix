# pineapple, built from this directory's crate, a library with no program
# yet, its tests run as it builds, reading sayings.json beside it.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "pineapple";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./sayings.json
      ./src
      ./tests
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Judges whether a saying is the safe word that stops Joel's agents, and what it reaches";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}
