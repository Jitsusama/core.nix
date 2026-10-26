# pi-packages, built from this directory, its tests run as it builds against
# a package in a local repository, with the git, node and pnpm it installs
# with.
{
  lib,
  rustPlatform,
  bash,
  git,
  nodejs,
  pnpm,
}:
rustPlatform.buildRustPackage {
  pname = "pi-packages";
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
