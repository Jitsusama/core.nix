# loopback-holders, built from this directory's crate. Its tests run as it
# builds, the program itself among them over a sysfs of their own. It
# takes its shape from the terminal kit (../../terminal-kit), so the source
# is cut from the modules folder holding both, and built from this one.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "loopback-holders";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../../terminal-kit/Cargo.toml
      ../../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/v4l2loopback/holders";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says which programs have a camera open, or stream from it";
    license = lib.licenses.mit;
    mainProgram = "loopback-holders";
    platforms = lib.platforms.linux;
  };
}
