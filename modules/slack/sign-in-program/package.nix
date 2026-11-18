# sign-in-program, built from this directory's crate, a library with no
# program, its tests run as it builds: each starts a small shell program
# as a stand-in sign-in.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "sign-in-program";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./keyring.sh
      ./src
      ./tests
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Runs the program a reader is given for Joel's sign-in, never waiting on it for good";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}
