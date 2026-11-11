# night-shift, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "night-shift";
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
    description = "Runs a heavy job at night, once, only while the machine has memory to spare";
    license = lib.licenses.mit;
    mainProgram = "night-shift";
    platforms = lib.platforms.linux;
  };
}
