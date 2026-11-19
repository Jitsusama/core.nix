# efficient-cores, which holds the efficient cores' clock ceiling or frees
# it (efficient-cores.sh says how, and nixos.nix why).
{
  coreutils,
  writeShellApplication,
}:
writeShellApplication {
  name = "efficient-cores";
  runtimeInputs = [ coreutils ];
  text = builtins.readFile ./efficient-cores.sh;
}
