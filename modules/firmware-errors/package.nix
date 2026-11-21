# firmware-errors, built from this directory, its tests run as it builds.
# It takes its shape from the terminal kit (../terminal-kit), so the
# source is cut from the folder holding both. Its tests run keep.sh over
# stand-ins (src/keeping.rs) and read back what it kept, to hold what the
# keeping writes to what the command reads.
{
  bash,
  coreutils,
  diffutils,
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "firmware-errors";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./keep.sh
      ./src
      ./tests
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/firmware-errors";
  # What keep.sh runs with in its unit, for the tests that run it.
  nativeCheckInputs = [
    bash
    coreutils
    diffutils
  ];
  cargoLock = {
    lockFile = ./Cargo.lock;
    # Intel's decoder is taken from its repository at a commit (Cargo.toml
    # says why), so its source is fixed by this hash.
    outputHashes."intel_crashlog-1.2.1" = "sha256-pI930uyBZpKOOuVwUoqZzSOUg/FZbAuwQUpIJYCdCvE=";
  };
  meta = {
    description = "Says why each kept boot's last one ended, from what the firmware handed it";
    license = lib.licenses.mit;
    mainProgram = "firmware-errors";
  };
}
