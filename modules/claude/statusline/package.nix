# Claude Code's status line, built from this directory's crate. Its tests
# run as it builds. It takes its shape and ink from the terminal kit
# (../../terminal-kit), so the source is cut from the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "claude-statusline";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../../terminal-kit/Cargo.toml
      ../../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/claude/statusline";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Claude Code's status line: the folder, the model and the context's size";
    license = lib.licenses.mit;
    mainProgram = "claude-statusline";
  };
}
