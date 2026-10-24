# The program core's VM tests start virtiofsd through, so it is refused
# syncfs(2), built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "without-syncfs";
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
    description = "Runs a program refused syncfs, answered as not implemented";
    license = lib.licenses.mit;
    mainProgram = "without-syncfs";
    platforms = lib.platforms.linux;
  };
}
