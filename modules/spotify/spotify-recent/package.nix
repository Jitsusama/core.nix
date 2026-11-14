# spotify-recent, built from this directory, its tests run as it builds,
# against fixtures and a stand-in for Spotify on the build machine. It takes
# its shape from the terminal kit (../../terminal-kit), so the source is cut
# from the modules folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "spotify-recent";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./fixtures
      ./src
      ./tests
      ../../terminal-kit/Cargo.toml
      ../../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/spotify/spotify-recent";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says what Spotify played lately, the playlists and liked songs, and plays one";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    mainProgram = "spotify-recent";
  };
}
