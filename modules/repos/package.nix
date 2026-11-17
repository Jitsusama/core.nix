# repos, built from this directory, its tests run as it builds. Its command
# line takes the shape every command's does (../terminal-kit), and its
# daemon serves as every daemon does (../daemon-serve), and its receptor is
# said as every daemon's is (../receptor-say), so the source is cut from the
# modules folder holding them, and built from this one.
{
  git,
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
      ./tests
      ../daemon-serve/Cargo.toml
      ../daemon-serve/src
      ../receptor-say/Cargo.toml
      ../receptor-say/src
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/repos";
  cargoLock.lockFile = ./Cargo.lock;
  # The counting's test counts over a small repository it builds with git,
  # and the weighing's weighs one git wrote and gathered.
  nativeCheckInputs = [ git ];
  meta = {
    description = "Lists every repository on the machine once, with its checkouts and trees";
    license = lib.licenses.mit;
    mainProgram = "repos";
  };
}
