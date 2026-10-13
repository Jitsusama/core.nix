# The firmware-settings command, which holds the firmware's settings to the
# ones a machine declares (nixos.nix).
{ writeShellApplication, coreutils }:
writeShellApplication {
  name = "firmware-settings";
  runtimeInputs = [ coreutils ];
  text = builtins.readFile ./firmware-settings.sh;
}
