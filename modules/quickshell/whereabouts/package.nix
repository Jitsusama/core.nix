# Where the laptop is, for the launcher's weather, built from this
# directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "whereabouts";
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
    description = "Says where the laptop is, from GeoClue, for the launcher's weather";
    license = lib.licenses.mit;
    mainProgram = "whereabouts";
    platforms = lib.platforms.linux;
  };
}
