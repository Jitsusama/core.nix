# network-revive, built from this directory, its tests run as it builds.
# It takes its shape from the terminal kit (../terminal-kit), so the
# source is cut from the modules folder holding both, and built from this
# one.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "network-revive";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/network-revive";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Gives an account's units another go when the network changes";
    license = lib.licenses.mit;
    mainProgram = "network-revive";
    platforms = lib.platforms.linux;
  };
}
