# chrome-recent-sites, built from this directory's crate with its tests run
# as it builds: the sites Chrome visited lately and the tabs it has open,
# for its Recent in the launcher.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "chrome-recent-sites";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Prints the sites Chrome visited lately, or the tabs it has open, newest first";
    license = lib.licenses.mit;
    mainProgram = "chrome-recent-sites";
    platforms = lib.platforms.linux;
  };
}
