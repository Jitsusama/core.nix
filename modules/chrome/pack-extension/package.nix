# chrome-pack-extension, built from this directory's crate with its tests run
# as it builds: packs a folder as a Chrome extension file, signed, for
# Chrome to install from a local file. It takes its shape from the terminal
# kit (../../terminal-kit), so the source is cut from the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "chrome-pack-extension";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../../terminal-kit/Cargo.toml
      ../../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/chrome/pack-extension";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Packs a folder as a signed Chrome extension file, printing its id";
    license = lib.licenses.mit;
    mainProgram = "chrome-pack-extension";
    platforms = lib.platforms.linux;
  };
}
