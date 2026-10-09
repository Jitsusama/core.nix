# Secure Boot with keys that never leave the machine, and a disk that unlocks
# only when the machine booted what it should. lanzaboote signs every
# generation it installs, and seals the disk's TPM key to a systemd-pcrlock
# policy it rewrites after every rebuild. How a machine gets there the first
# time is in docs/installing.md.
{ lanzaboote }:
{ lib, pkgs, ... }:
{
  imports = [ lanzaboote.nixosModules.lanzaboote ];

  # lanzaboote installs and signs systemd-boot itself.
  boot.loader.systemd-boot.enable = lib.mkForce false;
  boot.loader.efi.canTouchEfiVariables = true;
  # The TPM and the PIN are asked for by the systemd initrd.
  boot.initrd.systemd.enable = true;

  boot.lanzaboote = {
    enable = true;
    pkiBundle = "/var/lib/sbctl";

    # The first boot makes the keys, and systemd-boot enrols them into the
    # firmware on the next, so no key is ever made anywhere else.
    autoGenerateKeys.enable = true;
    autoEnrollKeys = {
      enable = true;
      autoReboot = true;
      # Only Joel's keys, so nothing Microsoft ever signed boots here, such as
      # a shim that shows a fake disk prompt to learn the PIN. The firmware
      # still runs the expansion ROMs it ran when the keys went in, by their
      # checksums from the TPM's event log. lanzaboote calls leaving
      # Microsoft's keys out a risk of bricking, because a graphics card's
      # ROM that isn't trusted leaves no picture; a laptop's picture comes
      # from its firmware, whose setup screen can always restore the
      # factory keys. A machine with a graphics card needs Microsoft's keys.
      includeMicrosoftKeys = false;
      includeChecksumsFromTPM = true;
      allowBrickingMyMachine = true;
    };

    # The disk's TPM key opens only for the same firmware (PCR 0), the same
    # boot loader and signed kernel, initrd and command line (PCR 4), and the
    # same Secure Boot keys and state (PCR 7). systemd-pcrlock covers at most
    # four generations.
    measuredBoot = {
      enable = true;
      pcrs = [
        0
        4
        7
      ];
    };
    configurationLimit = 4;
  };

  environment.systemPackages = [ pkgs.sbctl ];
}
