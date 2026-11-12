# The daemon that holds where the laptop is and the clock's zone, for the
# launcher, built from this directory, its tests run as it builds: its
# socket's among them (tests/daemon.rs). Those tests start it and ask it
# with a crate every daemon's tests share (../../daemon-client), and its
# rows take the form every daemon's do (../../ready-rows), so the source
# is cut from the modules folder holding them, and built from this one.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "whereabouts";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../../daemon-client/Cargo.toml
      ../../daemon-client/src
      ../../ready-rows/Cargo.toml
      ../../ready-rows/src
    ];
  };
  sourceRoot = "${src.name}/quickshell/whereabouts";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Holds where the laptop is, from GeoClue, and the clock's zone, over a socket";
    license = lib.licenses.mit;
    mainProgram = "location";
    platforms = lib.platforms.linux;
  };
}
