# last-seconds and last-seconds-keep, built from this directory, their tests
# run as they build. They take their shape from the terminal kit
# (../terminal-kit), so the source is cut from the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "last-seconds";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      # The recorder run whole and signalled as the sleep's hook does.
      ./tests/each_sleep_is_heard.rs
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/last-seconds";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says what the machine was doing in the seconds before each boot ended";
    license = lib.licenses.mit;
    mainProgram = "last-seconds";
  };
}
