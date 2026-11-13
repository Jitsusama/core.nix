# lag-watch, built from this directory, its tests run as it builds. It
# takes its shape from the terminal kit (../terminal-kit), so the source is
# cut from the folder holding both, and built from this one.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "lag-watch";
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
  sourceRoot = "${src.name}/lag-watch";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Keeps a standing watch on whether what Joel uses waits";
    license = lib.licenses.mit;
    mainProgram = "lag-watch";
    platforms = lib.platforms.linux;
  };
}
