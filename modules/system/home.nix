# system says how the whole system stands at once: the machines of the
# realm, the mycelium, the home network and the code (src/lib.rs says how).
# It asks each part's own command, mycelium, net and repos, as they're found
# on the account's path, so it keeps no daemon and no copy of theirs, and a
# machine without one of them says that part wasn't read.
{ pkgs, ... }:
{
  home.packages = [ (pkgs.callPackage ./package.nix { }) ];

  # So an agent asked how everything stands answers from the owners, in
  # words, worst first, as the terminal does.
  home.file.".pi/agent/skills/system/SKILL.md".source = ./SKILL.md;
}
