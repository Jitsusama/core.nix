# ir-camera, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "ir-camera";
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
    description = "Relays an IR camera's raw frames to a camera any program can open";
    license = lib.licenses.mit;
    mainProgram = "ir-camera";
    platforms = lib.platforms.linux;
  };
}
