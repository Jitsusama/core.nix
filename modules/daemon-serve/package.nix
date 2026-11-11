# daemon-serve, built from this directory's crate, a library with no
# program, its tests run as it builds. They ask it over a socket with the
# client every daemon's tests share (../daemon-client), so the source is
# cut from the modules folder holding both, and built from this one.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "daemon-serve";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../daemon-client/Cargo.toml
      ../daemon-client/src
    ];
  };
  sourceRoot = "${src.name}/daemon-serve";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Serves a daemon's records to the launcher over its socket";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}
