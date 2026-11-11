# ready-rows, built from this directory's crate, a library with no
# program, its tests run as it builds, the figures' among them reading
# numbers.json beside it.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "ready-rows";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./numbers.json
      ./src
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "The one form every daemon serves the launcher's rows in";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}
