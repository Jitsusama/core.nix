# bluetooth-links, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "bluetooth-links";
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
    description = "Says how each connected Bluetooth link is doing, for the launcher";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    mainProgram = "bluetooth-links";
  };
}
