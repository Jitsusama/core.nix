# The to-do daemon, built from this directory, its tests run as it builds.
# They run checks in sh with coreutils' timeout, as the daemon does.
{
  bash,
  coreutils,
  git,
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "to-do";
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
  nativeCheckInputs = [
    bash
    coreutils
    git
  ];
  meta = {
    description = "Runs each to-do's check and hears its signals, so a to-do clears by itself";
    license = lib.licenses.mit;
    mainProgram = "to-do";
    platforms = lib.platforms.linux;
  };
}
