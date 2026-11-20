# sessions-come-back, built from this directory, its tests run as it builds.
# It takes its shape from the terminal kit (../terminal-kit), so the source
# is cut from the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "sessions-come-back";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/sessions-come-back";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Brings a machine's pi sessions back after it restarts";
    license = lib.licenses.mit;
    mainProgram = "sessions-come-back";
    # A Mac is what runs it; Linux builds it for its tests and clippy.
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
