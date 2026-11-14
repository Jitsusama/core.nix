# The door pi's fenced commands reach GitHub through, built from this
# folder, its tests run as it builds (tests/door.rs). It takes its shape
# from the terminal kit (../../terminal-kit), so the source is cut from
# the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "pi-github-door";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./mutations.json
      ./src
      ./tests
      ../../terminal-kit/Cargo.toml
      ../../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/pi/github-door";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "The door pi's fenced commands reach GitHub through, adding the token only to calls it allows";
    license = lib.licenses.mit;
    mainProgram = "pi-github-door";
    platforms = lib.platforms.linux;
  };
}
