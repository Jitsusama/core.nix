# google, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "google";
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
    description = "Holds the Google account for the desktop: the meetings Joel has, for the shell";
    license = lib.licenses.mit;
    mainProgram = "google";
    # It serves the shell over a Unix socket, on the Linux desktop.
    platforms = lib.platforms.linux;
  };
}
