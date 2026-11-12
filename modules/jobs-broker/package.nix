# The jobs broker, built from this directory, its tests run as it builds.
# It serves with the serving side every daemon shares (../daemon-serve),
# and its tests ask it with the client every daemon's tests share
# (../daemon-client), so the source is cut from the modules folder holding
# all three, and built from this one.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "jobs-broker";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../daemon-client/Cargo.toml
      ../daemon-client/src
      ../daemon-serve/Cargo.toml
      ../daemon-serve/src
    ];
  };
  sourceRoot = "${src.name}/jobs-broker";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Makes a fenced pi command's own scope in agents' work, and says how units stand";
    license = lib.licenses.mit;
    mainProgram = "jobs-broker";
    platforms = lib.platforms.linux;
  };
}
