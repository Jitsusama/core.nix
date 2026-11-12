# pi-keys, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "pi-keys";
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
    description = "Samples what a key typed into the pi window Joel is at waits on";
    license = lib.licenses.mit;
    mainProgram = "pi-keys";
    platforms = lib.platforms.linux;
  };
}
