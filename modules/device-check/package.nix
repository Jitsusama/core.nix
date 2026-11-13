# The device-check command, which names the devices that have stopped working,
# and device-check-disk, which smartd runs to keep what a disk reports where
# device-check reads it (nixos.nix). Built from this directory, its tests run
# as it builds, with notify-send's path built in. Both take their shape from
# the terminal kit (../terminal-kit), so the source is cut from the folder
# holding both.
{
  lib,
  rustPlatform,
  libnotify,
}:
rustPlatform.buildRustPackage rec {
  pname = "device-check";
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
  sourceRoot = "${src.name}/device-check";
  cargoLock.lockFile = ./Cargo.lock;
  env.DEVICE_CHECK_NOTIFY_SEND = lib.getExe' libnotify "notify-send";
  meta = {
    description = "Names the devices that have stopped working";
    license = lib.licenses.mit;
    mainProgram = "device-check";
  };
}
