# elsewhere, built from this directory, its tests run as it builds. It takes
# its shape from the terminal kit (../terminal-kit), so the source is cut
# from the modules folder holding both, and built from this one.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "elsewhere";
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
  sourceRoot = "${src.name}/elsewhere";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Runs a heavy job wherever mycelium places it";
    license = lib.licenses.mit;
    mainProgram = "elsewhere";
    platforms = lib.platforms.unix;
  };
}
