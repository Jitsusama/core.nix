# daemon-client, built from this directory's crate, a library with no
# program; its own test runs as it builds, and the daemons' tests that
# name it run it too.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "daemon-client";
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
    description = "Starts a daemon of the launcher's interface for a test, and asks it";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}
