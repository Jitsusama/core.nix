# cadence, built from this directory, its tests run as it builds. `cadence
# serve` serves with the serving side every daemon shares (../daemon-serve)
# in the one form every daemon's rows take (../ready-rows), and its test
# asks it with the client every daemon's tests share (../daemon-client), so
# the source is cut from the modules folder holding all four, and built
# from this one.
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
      ../daemon-client/Cargo.toml
      ../daemon-client/src
      ../daemon-serve/Cargo.toml
      ../daemon-serve/src
      ../ready-rows/Cargo.toml
      ../ready-rows/src
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
