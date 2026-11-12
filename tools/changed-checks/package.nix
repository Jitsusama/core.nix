# changed-checks, for the dev shell, built from this directory's crate. Its
# tests run as it builds. On Linux it traces a base's VM tests' evaluation
# with strace, to see what they read; without it they evaluate every time.
{
  lib,
  rustPlatform,
  stdenv,
  strace,
}:
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
  env = lib.optionalAttrs stdenv.hostPlatform.isLinux {
    CHANGED_CHECKS_STRACE = "${strace}/bin/strace";
  };
  meta = {
    description = "Names the checks a change touches, by their derivations, and builds those";
    license = lib.licenses.mit;
    mainProgram = "changed-checks";
  };
}
