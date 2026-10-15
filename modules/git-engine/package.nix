# git-engine, built from this directory's crate. Its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "git-engine";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Answers the prompt's questions about a git repository, starting nothing";
    license = lib.licenses.mit;
    mainProgram = "git-engine";
    platforms = lib.platforms.unix;
  };
}
