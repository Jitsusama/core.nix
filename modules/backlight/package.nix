# backlight, built from this directory's crate. Its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "backlight";
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
    description = "Sets the screen's brightness for the light, and learns from the brightness picked";
    license = lib.licenses.mit;
    mainProgram = "backlight";
    platforms = lib.platforms.linux;
  };
}
