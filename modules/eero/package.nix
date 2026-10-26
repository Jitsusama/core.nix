# eero, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "eero";
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
    description = "Reads the home network's eero from eero's cloud, at home or away";
    license = lib.licenses.mit;
    mainProgram = "eero";
    # It seals its session with systemd's credential service.
    platforms = lib.platforms.linux;
  };
}
