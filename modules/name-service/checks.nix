# The checks of name-service, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  lib,
  self,
  ...
}:
{
  # Every NixOS machine's name service is nsncd with the patch, as the base
  # role brings it, so no machine loses a family's addresses.
  # vm-names-give-ipv4-when-ipv6-answers-first shows what the patch does.
  every-machine-asks-names-in-each-family =
    let
      machine = bareNixos [ self.nixosModules.base ];
      inherit (machine.config.services) nscd;
      patched = lib.any (
        patch: baseNameOf (toString patch) == "getai-asks-each-family.patch"
      ) machine.pkgs.nsncd.patches;
    in
    holds "every-machine-asks-names-in-each-family"
      "A NixOS machine's name service isn't nsncd asking for each address family on its own."
      (nscd.enable && nscd.enableNsncd && patched);
}
