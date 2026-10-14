# network-revive, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "network-revive";
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
    description = "Gives an account's units another go when the network changes";
    license = lib.licenses.mit;
    mainProgram = "network-revive";
    platforms = lib.platforms.linux;
  };
}
