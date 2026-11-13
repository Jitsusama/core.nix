# firmware-notice, built from this directory, its tests run as it builds. It
# takes its shape from the terminal kit (../terminal-kit), so the source is
# cut from the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "firmware-notice";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/firmware-notice";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says, once, each firmware update fwupd offers the machine";
    license = lib.licenses.mit;
    mainProgram = "firmware-notice";
    platforms = lib.platforms.linux;
  };
}
