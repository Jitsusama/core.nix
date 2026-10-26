# pause-for-calls, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "pause-for-calls";
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
    description = "Pauses Spotify while a call app records from a microphone";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    mainProgram = "pause-for-calls";
  };
}
