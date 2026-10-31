# The checks of name-service, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  lib,
  self,
  ...
}:
let
  machine = bareNixos [ self.nixosModules.base ];
  inherit (machine.config.services) nscd;
  patched = name: lib.any (patch: baseNameOf (toString patch) == name) machine.pkgs.nsncd.patches;
in
{
  # Every NixOS machine's name service is nsncd with the patch, as the base
  # role brings it, so no machine loses a family's addresses.
  # vm-names-give-ipv4-when-ipv6-answers-first shows what the patch does.
  every-machine-asks-names-in-each-family =
    holds "every-machine-asks-names-in-each-family"
      "A NixOS machine's name service isn't nsncd asking for each address family on its own."
      (nscd.enable && nscd.enableNsncd && patched "getai-asks-each-family.patch");
  # And with the patch that answers a .local name in time, so one absent
  # neighbour doesn't cut a program off from every .local name.
  # vm-names-give-ipv4-when-ipv6-answers-first shows that too.
  every-machine-answers-local-names-in-time =
    holds "every-machine-answers-local-names-in-time"
      "A NixOS machine's name service isn't nsncd answering .local names in time."
      (nscd.enable && nscd.enableNsncd && patched "local-names-answer-in-time.patch");
}
