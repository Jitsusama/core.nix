# lag-watch, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "lag-watch";
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
    description = "Keeps a standing watch on whether what Joel uses waits";
    license = lib.licenses.mit;
    mainProgram = "lag-watch";
    platforms = lib.platforms.linux;
  };
}
