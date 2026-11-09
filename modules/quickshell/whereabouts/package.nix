# The daemon that holds where the laptop is and the clock's zone, for the
# launcher, built from this directory, its tests run as it builds: its
# socket's among them (tests/daemon.rs).
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "whereabouts";
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
    description = "Holds where the laptop is, from GeoClue, and the clock's zone, over a socket";
    license = lib.licenses.mit;
    mainProgram = "whereabouts";
    platforms = lib.platforms.linux;
  };
}
