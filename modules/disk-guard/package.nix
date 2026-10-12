# The disk guard, built from this directory, its scenarios run as it builds.
{
  git,
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "disk-guard";
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
  # The predicate scenarios read a real checkout, through git as the guard does.
  nativeCheckInputs = [ git ];
  meta = {
    description = "Keeps a disk from filling without breaking a build";
    license = lib.licenses.mit;
    mainProgram = "disk-guard";
  };
}
