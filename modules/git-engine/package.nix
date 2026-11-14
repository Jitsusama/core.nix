# git-engine, built from this directory's crate. Its tests run as it builds.
# It takes its help and facts from the terminal kit (../terminal-kit), so the
# source is cut from the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "git-engine";
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
  sourceRoot = "source/git-engine";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Answers the prompt's questions about a git repository, starting nothing";
    license = lib.licenses.mit;
    mainProgram = "git-engine";
    platforms = lib.platforms.unix;
  };
}
