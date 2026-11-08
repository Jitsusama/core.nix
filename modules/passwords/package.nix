# The passwords daemon and command, built from this directory, its tests
# run as it builds.
{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "passwords";
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
    description = "Holds Joel's passwords ready for the shell and the command line, from each password manager";
    license = lib.licenses.mit;
    mainProgram = "passwords";
    platforms = lib.platforms.linux;
  };
}
