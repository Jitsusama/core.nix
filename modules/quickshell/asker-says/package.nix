# What a process asking for Joel's password says about itself, for polkit's
# prompt, built from this directory, its tests run as it builds. It takes its
# shape from the terminal kit (../../terminal-kit), so the source is cut from
# the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "asker-says";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../../terminal-kit/Cargo.toml
      ../../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/quickshell/asker-says";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says the quest and session a process asking for a password says it runs under";
    license = lib.licenses.mit;
    mainProgram = "asker-says";
    platforms = lib.platforms.linux;
  };
}
