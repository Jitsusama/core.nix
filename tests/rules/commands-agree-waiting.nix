# The commands that broke a point of docs/commands.md's convention before
# commands-agree, each with the points it still breaks, waiting while it comes
# to agree. The check fails on a command breaking a point it doesn't wait on,
# and on one waiting on a point it keeps, so the list only shrinks.
{
  merge-net = [ "exits" ];
  nix-offload = [
    "kit"
    "exits"
    "helps"
  ];
  pi-carry = [ "helps" ];
  pi-verbs = [ "helps" ];
  secret-asks = [
    "kit"
    "exits"
    "helps"
  ];
}
