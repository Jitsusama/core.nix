# cadence, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "cadence";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
      # Its tests read the keys spend-watch writes, which it copies.
      ../spend-watch/src/detect.rs
      ../spend-watch/src/collection.rs
    ];
  };
  sourceRoot = "${src.name}/cadence";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Watches the delivery cycle and asks the session keeping it fast when a target slips";
    license = lib.licenses.mit;
    mainProgram = "cadence";
    platforms = lib.platforms.linux;
  };
}
