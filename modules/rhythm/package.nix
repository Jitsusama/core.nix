# rhythm, built from this directory's crate. Its tests run as it builds.
# The sun's reckoning is a crate it shares with the backlight (../sun), so
# the source is cut from the modules folder holding both, and built from
# this one.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "rhythm";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../sun/Cargo.toml
      ../sun/src
    ];
  };
  sourceRoot = "${src.name}/rhythm";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Learns when Joel stops for the night, from when he's at the machine";
    license = lib.licenses.mit;
    mainProgram = "rhythm";
    platforms = lib.platforms.linux;
  };
}
