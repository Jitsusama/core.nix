# The program that puts the VMs of Nix's checks ahead of the builds beside
# them, built from this directory, its tests run as it builds. It takes its
# shape from the terminal kit (../../terminal-kit), so the source is cut
# from the folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "vm-first";
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
  sourceRoot = "${src.name}/nix-daemon/vm-first";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Puts the VMs of Nix's checks ahead of the builds beside them";
    license = lib.licenses.mit;
    mainProgram = "vm-first";
  };
}
