# Claude Code's status line, built from this directory's crate. Its tests
# run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "claude-statusline";
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
  meta = {
    description = "Claude Code's status line: the folder, the model and the context's size";
    license = lib.licenses.mit;
    mainProgram = "claude-statusline";
  };
}
