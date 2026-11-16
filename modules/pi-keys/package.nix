# pi-keys, built from this directory, its tests run as it builds. Its
# shape comes from the terminal kit (../terminal-kit), so the source is
# cut from the modules folder holding both, and built from this one.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "pi-keys";
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
  sourceRoot = "${src.name}/pi-keys";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Samples what a key typed into the pi window Joel is at waits on";
    license = lib.licenses.mit;
    mainProgram = "pi-keys";
    platforms = lib.platforms.linux;
  };
}
