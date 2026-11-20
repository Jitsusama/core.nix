# efficient-cores, which holds the efficient cores' clock ceiling or frees
# it (efficient-cores.sh says how, and nixos.nix why).
{
  coreutils,
  util-linux,
  writeShellApplication,
}:
writeShellApplication {
  name = "efficient-cores";
  # util-linux for flock.
  runtimeInputs = [
    coreutils
    util-linux
  ];
  text = builtins.readFile ./efficient-cores.sh;
}
