# The checks of lanzaboote, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  lib,
  pkgs,
  self,
  ...
}:
{
  # firmware-update.sh frees PCR 0 only for the boot entry fwupd points
  # BootNext at, as tests/firmware-update.sh says, with stand-ins for
  # efibootmgr and systemd-pcrlock. vm-secure-boot-installs shows it at a
  # real shutdown, in minutes; this holds which BootNext it acts on, in
  # seconds.
  a-firmware-update-frees-pcr-0-only-for-fwupds-entry =
    pkgs.runCommandLocal "a-firmware-update-frees-pcr-0-only-for-fwupds-entry" { }
      ''
        ${lib.getExe pkgs.bash} ${./tests/firmware-update.sh} ${./firmware-update.sh}
        touch $out
      '';

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

  # The wrappers unit stays active once it's run, so the units measured
  # boot's wait for sysinit.target pulls in again can't run it past its
  # start limit and leave it failed.
  wrappers-run-once-a-boot =
    let
      machine = bareNixos [ self.nixosModules.lanzaboote ];
    in
    holds "wrappers-run-once-a-boot"
      "the SUID wrappers' unit can run again while measured boot holds sysinit.target back, and end failed at its start limit."
      (machine.config.systemd.services.suid-sgid-wrappers.serviceConfig.RemainAfterExit or false);
}
