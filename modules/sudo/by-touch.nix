# sudo-by-touch, which Joel's terminal runs for sudo (nixos.nix), as
# by-touch.sh says why. run0 and sudo are given, so the check can build it
# in front of stand-ins.
{
  run0,
  sudo,
  writeShellApplication,
}:
writeShellApplication {
  name = "sudo-by-touch";
  text = builtins.replaceStrings [ "@run0@" "@sudo@" ] [ run0 sudo ] (
    builtins.readFile ./by-touch.sh
  );
}
