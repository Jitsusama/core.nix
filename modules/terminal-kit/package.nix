# terminal-kit, built from this directory's crate, a library with no
# program, its tests run as it builds, the theme's reading among them
# over Osaka Jade's colours in tests/.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "terminal-kit";
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
    description = "The shape every command takes, for a person in a terminal and a machine in a pipe";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
