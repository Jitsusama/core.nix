# key-counts, built from this directory, its tests run as it builds. It
# takes its shape from the terminal kit (../terminal-kit), so the source is
# cut from the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "key-counts";
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
  sourceRoot = "${src.name}/key-counts";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Counts each key pressed, by keyboard, layer and modifiers, and nothing typed";
    license = lib.licenses.mit;
    mainProgram = "key-counts";
    platforms = lib.platforms.linux;
  };
}
