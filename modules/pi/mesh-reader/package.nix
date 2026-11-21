# The mesh reader, built from this folder with its tests run (src/lib.rs
# says why). It takes its help from the terminal kit
# (../../terminal-kit), so the source is cut from the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "pi-mesh-reader";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../../terminal-kit/Cargo.toml
      ../../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/pi/mesh-reader";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Reads each folder of the mesh's once for every pi process that asks";
    license = lib.licenses.mit;
    mainProgram = "pi-mesh-reader";
    platforms = lib.platforms.linux;
  };
}
