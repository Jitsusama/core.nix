# The program that tells the shaped machine on this line what this one
# sends up, built from this directory, its tests run as it builds. It takes
# its shape from the terminal kit (../terminal-kit), so the source is cut
# from the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "house-report";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/house-report";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Tells the shaped machine on this line what this one sends up";
    license = lib.licenses.mit;
    mainProgram = "house-report";
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
