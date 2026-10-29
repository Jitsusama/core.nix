# chrome-pack-extension, built from this directory's crate with its tests run
# as it builds: packs a folder as a Chrome extension file, signed, for
# Chrome to install from a local file.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "chrome-pack-extension";
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
    description = "Packs a folder as a signed Chrome extension file, printing its id";
    license = lib.licenses.mit;
    mainProgram = "chrome-pack-extension";
    platforms = lib.platforms.linux;
  };
}
