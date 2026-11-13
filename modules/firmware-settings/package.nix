# The firmware-settings command, which holds the firmware's settings to the
# ones a machine declares (nixos.nix), built from this directory, its tests
# run as it builds. It takes its shape from the terminal kit (../terminal-kit),
# so the source is cut from the folder holding both.
{
  lib,
  rustPlatform,
  coreutils,
}:
rustPlatform.buildRustPackage rec {
  pname = "firmware-settings";
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
  sourceRoot = "${src.name}/firmware-settings";
  cargoLock.lockFile = ./Cargo.lock;
  # The stty it turns the terminal's echo off with while the password is typed.
  env.STTY = lib.getExe' coreutils "stty";
  meta = {
    description = "Holds the firmware's settings to the ones the machine declares";
    license = lib.licenses.mit;
    mainProgram = "firmware-settings";
  };
}
