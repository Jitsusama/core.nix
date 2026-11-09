# backlight, built from this directory's crate. Its tests run as it builds.
# The sun's reckoning is a crate it shares with the launcher's calendars
# (../sun), so the source is cut from the modules folder holding both, and
# built from this one.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "backlight";
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
  sourceRoot = "${src.name}/backlight";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Sets the screen's brightness and warmth for the light, learning from what's picked";
    license = lib.licenses.mit;
    mainProgram = "backlight";
    platforms = lib.platforms.linux;
  };
}
