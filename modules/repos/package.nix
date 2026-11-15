# repos, built from this directory, its tests run as it builds. Its command
# line takes the shape every command's does (../terminal-kit), so the
# source is cut from the modules folder holding it, and built from this one.
{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "repos";
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
  sourceRoot = "${src.name}/repos";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Lists every repository on the machine once, with its checkouts and trees";
    license = lib.licenses.mit;
    mainProgram = "repos";
  };
}
