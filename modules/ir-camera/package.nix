# ir-camera, built from this directory, its tests run as it builds. It takes
# its shape from the terminal kit (../terminal-kit), so the source is cut from
# the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "ir-camera";
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
  sourceRoot = "${src.name}/ir-camera";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Relays an IR camera's raw frames to a camera any program can open";
    license = lib.licenses.mit;
    mainProgram = "ir-camera";
    platforms = lib.platforms.linux;
  };
}
