# The device-check command, which names the devices that have stopped working,
# and device-check-disk, which smartd runs to keep what a disk reports where
# device-check reads it (nixos.nix).
{
  symlinkJoin,
  writeShellApplication,
  coreutils,
  findutils,
  libnotify,
}:
let
  deviceCheck = writeShellApplication {
    name = "device-check";
    runtimeInputs = [
      coreutils
      findutils
      libnotify
    ];
    text = builtins.readFile ./device-check.sh;
  };
  disk = writeShellApplication {
    name = "device-check-disk";
    runtimeInputs = [ coreutils ];
    text = builtins.readFile ./device-check-disk.sh;
  };
in
symlinkJoin {
  name = "device-check";
  paths = [
    deviceCheck
    disk
  ];
  meta.mainProgram = "device-check";
}
