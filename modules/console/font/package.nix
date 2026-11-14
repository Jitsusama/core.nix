# The tool that draws the console's font from a face, built from this
# directory, its tests run as it builds. It takes its shape from the terminal
# kit (../../terminal-kit), so the source is cut from the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "console-font";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../../terminal-kit/Cargo.toml
      ../../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/console/font";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Draws a console font from an OpenType face";
    license = lib.licenses.mit;
    mainProgram = "console-font";
  };
}
