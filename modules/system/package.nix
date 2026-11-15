# system, built from this directory, its tests run as it builds. Its command
# line takes the shape every command's does (../terminal-kit), so the
# source is cut from the modules folder holding it, and built from this one.
{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "system";
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
  sourceRoot = "${src.name}/system";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says how the whole system stands: the machines, the mycelium, the network and the code";
    license = lib.licenses.mit;
    mainProgram = "system";
  };
}
