# The Nix builders the machine offloads to, which the home realm's node
# shows beside its machines (home.nix): the lists nix-offload's lines account
# writes (../nix-offload/nixos.nix), the standing and beside builders dialled
# at most every five minutes, and the lent one never, since it costs while it
# runs; its lender's health, in the folder its line's host names, says how it
# stands. A list not there names none, so a machine that offloads nothing
# shows none. Linux only, since a Mac has no such lists.
[
  "--builders"
  "standing=/run/nix-offload-lines/machines-with-standing"
  "--builders"
  "beside=/run/nix-offload-lines/machines-beside"
  "--lent-builders"
  "lent=/run/nix-offload-lines/machines-for-anything"
  "--lent-health"
  "/run/{host}/health.json"
]
