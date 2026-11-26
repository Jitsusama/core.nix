# camera-guard, built from this directory, its tests run as it builds. It
# takes its shape from the terminal kit (../terminal-kit), so the source is
# cut from the modules folder holding both, and built from this one.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "camera-guard";
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
  sourceRoot = "${src.name}/camera-guard";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Keeps agents from opening a camera near Joel's meetings";
    license = lib.licenses.mit;
    mainProgram = "camera-guard";
    platforms = lib.platforms.linux;
  };
}
