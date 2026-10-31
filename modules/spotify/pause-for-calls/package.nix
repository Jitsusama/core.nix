# pause-for-calls, built from this directory, its tests run as it builds.
# It takes what counts as a call from modules/calls' crate, so the source
# holds both, from modules, and the build runs in this one.
{ lib, rustPlatform }:
let
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../../calls/Cargo.toml
      ../../calls/src
    ];
  };
in
rustPlatform.buildRustPackage {
  pname = "pause-for-calls";
  version = "0.1.0";
  inherit src;
  sourceRoot = "${src.name}/spotify/pause-for-calls";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Pauses Spotify while a call app records from a microphone";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    mainProgram = "pause-for-calls";
  };
}
