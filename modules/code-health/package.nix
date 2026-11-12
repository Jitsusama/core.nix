# code-health, built from this directory, its tests run as it builds, with
# git for the one that times a repository's merges.
{
  git,
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "code-health";
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
  nativeCheckInputs = [ git ];
  meta = {
    description = "Writes the day's figures of how messy each repository it's given is";
    license = lib.licenses.mit;
    mainProgram = "code-health";
  };
}
