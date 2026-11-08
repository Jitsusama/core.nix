# warp, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "warp";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      # The tests read warp-cli's answers and the record drawn from them.
      ./contract
      ./warp.json
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says how WARP's link stands, read from warp-cli";
    license = lib.licenses.mit;
    mainProgram = "warp";
  };
}
