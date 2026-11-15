# system says how the whole system stands at once: the machines of the
# realm, the mycelium, the home network and the code (src/lib.rs says how).
# It asks each part's own command, mycelium, net and repos, as they're found
# on the account's path, so it keeps no daemon, no record and no copy of
# theirs, and a machine without one of them says that part wasn't read.
{ pkgs, ... }:
{
  home.packages = [ (pkgs.callPackage ./package.nix { }) ];
}
