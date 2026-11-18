# elsewhere, built from this directory, its tests run as it builds. It takes
# its shape from the terminal kit (../terminal-kit), so the source is cut
# from the modules folder holding both, and built from this one.
{
  dash,
  git,
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "elsewhere";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/elsewhere";
  cargoLock.lockFile = ./Cargo.lock;
  # The tests track a job's files, since only tracked files travel.
  nativeCheckInputs = [ git ];
  # The far line runs under dash as the tests check it, the strictest sh a
  # far machine may have, so a bash-only word in it fails here.
  env.ELSEWHERE_FAR_SH = lib.getExe dash;
  meta = {
    description = "Runs a heavy job wherever mycelium places it";
    license = lib.licenses.mit;
    mainProgram = "elsewhere";
    platforms = lib.platforms.unix;
  };
}
