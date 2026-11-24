# writeback-goal, which sets a disk's writeback throttle goal and gives back
# what it had (writeback-goal.sh says how), built once for nixos.nix and its
# test.
{ coreutils, writeShellApplication }:
writeShellApplication {
  name = "writeback-goal";
  runtimeInputs = [ coreutils ];
  text = builtins.readFile ./writeback-goal.sh;
}
