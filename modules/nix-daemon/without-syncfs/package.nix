# The program core's VM tests start virtiofsd through, so it is refused
# syncfs(2), built from this directory, its tests run as it builds. It takes
# its shape from the terminal kit (../../terminal-kit), so the source is cut
# from the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "without-syncfs";
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
  sourceRoot = "${src.name}/nix-daemon/without-syncfs";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Runs a program refused syncfs, answered as not implemented";
    license = lib.licenses.mit;
    mainProgram = "without-syncfs";
    platforms = lib.platforms.linux;
  };
}
