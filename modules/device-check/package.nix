# The device-check command, which names the devices that have stopped working
# (nixos.nix).
{
  writeShellApplication,
  coreutils,
  findutils,
  libnotify,
}:
writeShellApplication {
  name = "device-check";
  runtimeInputs = [
    coreutils
    findutils
    libnotify
  ];
  text = builtins.readFile ./device-check.sh;
}
