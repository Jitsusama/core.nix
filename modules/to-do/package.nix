# The to-do daemon, built from this directory, its tests run as it builds.
# They run checks in sh, as the daemon does, with coreutils' sleep, rm and
# touch standing for checks that take a while, pass only once or leave a
# mark, and git for a switch's commit. It serves with the serving side
# every daemon shares (../daemon-serve), and its socket's tests ask it
# with the client every daemon's tests share (../daemon-client), so the
# source is cut from the modules folder holding all three, and built
# from this one.
{
  bash,
  coreutils,
  git,
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "to-do";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../daemon-client/Cargo.toml
      ../daemon-client/src
      ../daemon-serve/Cargo.toml
      ../daemon-serve/src
    ];
  };
  sourceRoot = "${src.name}/to-do";
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
