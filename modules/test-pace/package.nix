# test-pace, built from this directory's crate, a library with no
# program, its tests run as it builds. Crates whose tests bound how long
# a step took name it as a dev-dependency by path and add this folder to
# their own fileset.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "test-pace";
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
    description = "How long a timed step of a test may take on a host as busy as this one";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
