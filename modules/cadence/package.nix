# cadence, built from this directory, its tests run as it builds.
{
  git,
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "cadence";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      # The definition of good, which the hour holds its measures to.
      ./good.md
      ./src
      ./tests
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/cadence";
  cargoLock.lockFile = ./Cargo.lock;
  # The tests read which versions of a host patch carry a change from a
  # history of their own, as cadence reads core's.
  nativeCheckInputs = [ git ];
  meta = {
    description = "Watches the delivery cycle and asks the session keeping it fast when a target slips";
    license = lib.licenses.mit;
    mainProgram = "cadence";
    platforms = lib.platforms.linux;
  };
}
