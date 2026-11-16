# receptor-say, built from this directory's crate, a library with no
# program, its tests run as it builds. It needs nothing beyond std, so its
# source is this folder alone.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "receptor-say";
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
    description = "Says how a daemon's outside source stands to mycelium, in counts";
    license = lib.licenses.mit;
    # The daemons that say receptors run on the Linux desktop.
    platforms = lib.platforms.linux;
  };
}
