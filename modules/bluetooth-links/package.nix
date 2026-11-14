# bluetooth-links, built from this directory, its tests run as it builds.
# It takes its shape from the terminal kit (../terminal-kit), so the source
# is cut from the modules folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "bluetooth-links";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/bluetooth-links";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says how each connected Bluetooth link is doing, for the launcher";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    mainProgram = "bluetooth-links";
  };
}
