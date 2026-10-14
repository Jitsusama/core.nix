# spend-watch, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "spend-watch";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      # The feed contract's examples, which its tests read.
      ./contract
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Keeps an account told what its AI use costs";
    license = lib.licenses.mit;
    mainProgram = "spend-watch";
    platforms = lib.platforms.linux;
  };
}
