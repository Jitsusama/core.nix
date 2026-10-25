# load-proof, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "load-proof";
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
    description = "Says whether what Joel uses still feels idle under full load";
    license = lib.licenses.mit;
    mainProgram = "load-proof";
    platforms = lib.platforms.linux;
  };
}
