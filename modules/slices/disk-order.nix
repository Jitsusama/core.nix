# disk-order, which shares a disk by the slices' io.weight through io.cost
# (disk-order.sh says how), built once for nixos.nix and its test.
{
  coreutils,
  findutils,
  gnugrep,
  writeShellApplication,
}:
writeShellApplication {
  name = "disk-order";
  runtimeInputs = [
    coreutils
    findutils
    gnugrep
  ];
  text = builtins.readFile ./disk-order.sh;
}
