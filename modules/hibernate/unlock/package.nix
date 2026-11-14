# hibernate-unlock, built from this directory, its tests run as it builds.
# It takes its shape from the terminal kit (../../terminal-kit), so the
# source is cut from the modules folder holding both, and built from this
# one.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "hibernate-unlock";
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
  sourceRoot = "${src.name}/hibernate/unlock";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Lets the lock go after a resume from hibernation, and rescues one that rolled back";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    mainProgram = "hibernate-unlock";
  };
}
