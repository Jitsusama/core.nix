# pain-points, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "pain-points";
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
    description = "Names the interactions with the launcher that cost Joel most today";
    license = lib.licenses.mit;
    mainProgram = "pain-points";
    platforms = lib.platforms.linux;
  };
}
