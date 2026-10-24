# changed-checks, for the dev shell, built from this directory's crate. Its
# tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "changed-checks";
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
    description = "Names the checks a change touches, by their derivations, and builds those";
    license = lib.licenses.mit;
    mainProgram = "changed-checks";
  };
}
