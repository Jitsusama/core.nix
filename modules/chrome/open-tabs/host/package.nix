# chrome-open-tabs, built from this directory's crate with its tests run as
# it builds: Chrome's open tabs for the launcher, through our extension.
# It takes its shape from the terminal kit (../../../terminal-kit), so the
# source is cut from the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "chrome-open-tabs";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../../../terminal-kit/Cargo.toml
      ../../../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/chrome/open-tabs/host";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says which tabs are open in Chrome, and goes to or closes one";
    license = lib.licenses.mit;
    mainProgram = "chrome-open-tabs";
    platforms = lib.platforms.linux;
  };
}
