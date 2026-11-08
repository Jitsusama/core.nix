# spotify-recent, built from this directory, its tests run as it builds,
# against fixtures and a stand-in for Spotify on the build machine.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "spotify-recent";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./fixtures
      ./src
      ./tests
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says what Spotify played lately, the playlists and liked songs, and plays one";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    mainProgram = "spotify-recent";
  };
}
