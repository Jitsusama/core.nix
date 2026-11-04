# merge-net, built from this directory's crate. Its tests run as it builds.
# The quarantine list isn't part of it: the net reads it from main, so a
# test put on it moves no check.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "merge-net";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./must-run.txt
      ./src
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Runs the VM tests each merge to main changed, and reverts a merge that breaks one";
    license = lib.licenses.mit;
    mainProgram = "merge-net";
  };
}
