# firmware-notice, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "firmware-notice";
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
    description = "Says, once, each firmware update fwupd offers the machine";
    license = lib.licenses.mit;
    mainProgram = "firmware-notice";
    platforms = lib.platforms.linux;
  };
}
