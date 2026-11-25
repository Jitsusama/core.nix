# The local sites' server, built from this directory, its tests run as it
# builds. It follows a daemon with the watch every daemon's clients share
# (../daemon-serve) and starts as every command of ours does
# (../terminal-kit), so the source is cut from the modules folder holding
# all three, and built from this one.
{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "local-sites";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests/serve.rs
      ./tests/answer.rs
      ../daemon-serve/Cargo.toml
      ../daemon-serve/src
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/local-sites";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Serves Joel's local sites on loopback, each live page fed by its daemon";
    license = lib.licenses.mit;
    mainProgram = "local-sites";
    platforms = lib.platforms.linux;
  };
}
