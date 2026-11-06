# key-counts, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "key-counts";
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
    description = "Counts each key pressed, by keyboard, layer and modifiers, and nothing typed";
    license = lib.licenses.mit;
    mainProgram = "key-counts";
    platforms = lib.platforms.linux;
  };
}
