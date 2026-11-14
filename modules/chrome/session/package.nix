# chrome-session, built from this directory's crate with its tests run as
# it builds: what Chrome has open now, read from its session file. The
# program takes its shape from the terminal kit (../../terminal-kit), so
# the source is cut from the modules folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "chrome-session";
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
  sourceRoot = "${src.name}/chrome/session";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Prints what Chrome has open now, read from its session file";
    license = lib.licenses.mit;
    mainProgram = "chrome-session";
    platforms = lib.platforms.linux;
  };
}
