# chrome-recent-sites, built from this directory's crate with its tests run
# as it builds: the sites Chrome visited lately and its downloads, for its
# Recent in the launcher. It holds them to the rule of which pages are the
# web's that Chrome's open pages are held to (../session), so the source is
# cut from the folder holding both, and built from this one.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "chrome-recent-sites";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../session/Cargo.toml
      ../session/src
    ];
  };
  sourceRoot = "${src.name}/recent-sites";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Prints the sites Chrome visited lately, or its downloads, newest first";
    license = lib.licenses.mit;
    mainProgram = "chrome-recent-sites";
    platforms = lib.platforms.linux;
  };
}
