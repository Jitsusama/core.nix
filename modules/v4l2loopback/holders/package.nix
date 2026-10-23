# loopback-holders, built from this directory's crate. Its tests run as it
# builds, the program itself among them over a sysfs of their own.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "loopback-holders";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says which programs have a camera open, or stream from it";
    license = lib.licenses.mit;
    mainProgram = "loopback-holders";
    platforms = lib.platforms.linux;
  };
}
