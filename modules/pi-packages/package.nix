# pi-packages, built from this directory, its tests run as it builds against
# a package in a local repository, with the git, node and pnpm it installs
# with. It takes its shape from the terminal kit (../terminal-kit), so the
# source is cut from the folder holding both.
{
  lib,
  rustPlatform,
  bash,
  git,
  nodejs,
  pnpm,
}:
rustPlatform.buildRustPackage rec {
  pname = "pi-packages";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/pi-packages";
  cargoLock.lockFile = ./Cargo.lock;
  nativeCheckInputs = [
    bash
    git
    nodejs
    pnpm
  ];
  meta = {
    description = "Installs each of pi's packages as a whole version and swaps it in";
    license = lib.licenses.mit;
    mainProgram = "pi-packages";
    platforms = lib.platforms.unix;
  };
}
