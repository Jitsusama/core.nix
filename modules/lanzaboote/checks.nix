# The checks of lanzaboote, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  self,
  ...
}:
{
  # Every switch keeps the generation the machine booted on the ESP, so
  # systemd-pcrlock can always account for the running boot and the disk's
  # policy keeps PCR 4.
  booted-generation-stays-installed =
    let
      machine = bareNixos [ self.nixosModules.lanzaboote ];
    in
    holds "booted-generation-stays-installed"
      "lanzaboote can remove the booted generation, which drops PCR 4 from the disk's policy."
      (machine.config.boot.lanzaboote.protectedSystem == "/run/booted-system");
}
