# The program that puts the VMs of Nix's checks ahead of the builds beside
# them, built from this directory, its tests run as it builds.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "vm-first";
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
    description = "Puts the VMs of Nix's checks ahead of the builds beside them";
    license = lib.licenses.mit;
    mainProgram = "vm-first";
  };
}
