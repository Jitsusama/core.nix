# Secure Boot with keys that never leave the machine, and a disk that unlocks
# only when the machine booted what it should. lanzaboote signs every
# generation it installs, and seals the disk's TPM key to a systemd-pcrlock
# policy it rewrites after every rebuild. How a machine gets there the first
# time is in docs/installing.md.
{ lanzaboote }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  enrolment = config.boot.lanzaboote.autoEnrollKeys;

  # Leaving Microsoft's keys out makes lanzaboote pass sbctl
  # --yes-this-might-brick-my-machine, which also skips sbctl's refusal to
  # enrol keys that would stop the firmware running an option ROM it ran
  # this boot. This asks sbctl that question on its own. It exports to a
  # scratch directory, which needs no setup mode, so it changes nothing.
  optionRomCheck = pkgs.writeShellApplication {
    name = "option-rom-check";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.sbctl
    ];
    text = ''
      scratch="$(mktemp -d)"
      trap 'rm -rf "$scratch"' EXIT
      if ! output="$(cd "$scratch" && sbctl enroll-keys --export auth 2>&1)"; then
        printf '%s\n' "$output" >&2
        case "$output" in
        *"Found OptionROM"*)
          echo "The firmware ran an option ROM, so no keys were enrolled. Set" \
            "boot.lanzaboote.autoEnrollKeys.includeChecksumsFromTPM to keep running it," \
            "or includeMicrosoftKeys for a graphics card." >&2
          ;;
        esac
        exit 1
      fi
    '';
  };
in
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
      # a shim that shows a fake disk prompt to learn the PIN. lanzaboote
      # calls leaving Microsoft's keys out a risk of bricking, because a
      # graphics card's ROM that isn't trusted leaves no picture; a laptop's
      # picture comes from its firmware, whose setup screen can always
      # restore the factory keys. A machine with a graphics card needs
      # Microsoft's keys.
      includeMicrosoftKeys = false;
      allowBrickingMyMachine = true;
      # A machine whose firmware runs option ROMs sets this, so the firmware
      # keeps running them by their checksums from the TPM's event log. It
      # can't be on everywhere: sbctl refuses to enrol with it on a machine
      # whose firmware ran none. The check below says which one a machine
      # is.
      includeChecksumsFromTPM = lib.mkDefault false;
    };

    # The disk's TPM key opens only for the same firmware (PCR 0), the same
    # boot loader and signed kernel, initrd and command line (PCR 4), and the
    # same Secure Boot keys and state (PCR 7). systemd-pcrlock covers at most
    # four generations, and one of them is always the booted one: pcrlock
    # leaves PCR 4 out of the policy when it can't account for the boot it's
    # running in, until the next reboot.
    protectedSystem = "/run/booted-system";
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

  # Without checksums or Microsoft's keys, the firmware would stop running
  # any option ROM it runs now, so enrolment goes ahead only when it runs
  # none. A machine that does run one keeps its factory keys and is told
  # what to set.
  systemd.services.prepare-sb-auto-enroll.preStart = lib.mkIf (
    enrolment.enable && !enrolment.includeMicrosoftKeys && !enrolment.includeChecksumsFromTPM
  ) (lib.getExe optionRomCheck);
  # The install check runs it on a machine with checksums on, to see it
  # refuse an option ROM.
  system.build.optionRomCheck = optionRomCheck;

  environment.systemPackages = [ pkgs.sbctl ];
}
