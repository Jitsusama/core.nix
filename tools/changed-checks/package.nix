# changed-checks, for the dev shell, built from this directory's crate. Its
# tests run as it builds. On Linux it traces a base's VM tests' evaluation
# with strace, to see what they read; without it they evaluate every time.
# It takes its shape from the terminal kit (../../modules/terminal-kit), so
# the source is cut from the folder holding both.
{
  lib,
  rustPlatform,
  stdenv,
  strace,
}:
rustPlatform.buildRustPackage rec {
  pname = "changed-checks";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../../modules/terminal-kit/Cargo.toml
      ../../modules/terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/tools/changed-checks";
  cargoLock.lockFile = ./Cargo.lock;
  env = lib.optionalAttrs stdenv.hostPlatform.isLinux {
    CHANGED_CHECKS_STRACE = "${strace}/bin/strace";
  };
  meta = {
    description = "Names the checks a change touches, by their derivations, and builds those";
    license = lib.licenses.mit;
    mainProgram = "changed-checks";
  };
}
