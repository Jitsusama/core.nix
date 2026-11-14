# pineapple (Joel's own command for stops) and pineapple-slack, built
# from this directory's crate, its tests run as it builds, reading
# sayings.json beside it. It reads Slack
# through Slack's own module (../slack), which reads through the pieces
# every daemon shares, and takes its shape from the terminal kit
# (../terminal-kit), so the source is cut from the modules folder holding
# them all, and built from this one.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "pineapple";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./sayings.json
      ./src
      ./tests
      ../slack/Cargo.toml
      ../slack/src
      ../daemon-serve/Cargo.toml
      ../daemon-serve/src
      ../ready-rows/Cargo.toml
      ../ready-rows/src
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/pineapple";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Judges whether a saying is the safe word that stops Joel's agents, and reads Slack for it";
    license = lib.licenses.mit;
    mainProgram = "pineapple-slack";
    platforms = lib.platforms.linux;
  };
}
