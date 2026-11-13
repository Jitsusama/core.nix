# The disk guard, built from this directory, its scenarios run as it builds.
# Its command line takes its shape from the terminal kit (../terminal-kit),
# so the source is cut from the folder holding both.
{
  git,
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "disk-guard";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      # The scenarios alone: the check scripts beside them aren't the crate's.
      ./tests/guard
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/disk-guard";
  cargoLock.lockFile = ./Cargo.lock;
  # The predicate scenarios read a real checkout, through git as the guard does.
  nativeCheckInputs = [ git ];
  # The scenarios make, settle and delete thousands of files, so they run on
  # a tmpfs where Linux has one, rather than commit the build machine's own
  # disk with every pass.
  preCheck = ''
    if [ -d /dev/shm ]; then
      TMPDIR=$(mktemp -d -p /dev/shm disk-guard.XXXXXX)
      export TMPDIR
    fi
  '';
  meta = {
    description = "Keeps a disk from filling without breaking a build";
    license = lib.licenses.mit;
    mainProgram = "disk-guard";
  };
}
