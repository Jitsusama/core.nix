# chrome-open-tabs, built from this directory's crate with its tests run as
# it builds: Chrome's open tabs for the launcher, through our extension.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "chrome-open-tabs";
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
    description = "Says which tabs are open in Chrome, and goes to or closes one";
    license = lib.licenses.mit;
    mainProgram = "chrome-open-tabs";
    platforms = lib.platforms.linux;
  };
}
