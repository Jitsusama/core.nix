# sun, built from this directory's crate, a library with no program of its
# own, so building it is running its tests against NOAA's tables.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "sun";
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
    description = "Says when the sun rises and sets, as NOAA reckons it";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}
