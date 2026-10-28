# hibernate-unlock, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "hibernate-unlock";
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
    description = "Lets the lock go after a resume from hibernation, which asked at the disk";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    mainProgram = "hibernate-unlock";
  };
}
