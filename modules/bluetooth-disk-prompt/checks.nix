# The checks of bluetooth-disk-prompt, which checks.nix gathers with every
# other folder's; its opening comment says what each kind of check proves.
# The disk prompt is the systemd initrd's, so both are Linux's alone.
{
  bareNixos,
  lib,
  onLinux,
  pkgs,
  self,
  ...
}:
let
  # The initrd takes each file its hardware names from the machine's
  # firmware, compressed or not, and skips one it can't find without a
  # word; a controller with no firmware never comes up, and no keyboard
  # types. So each file named has to be there.
  firmwareReaches =
    name: hardware:
    let
      machine = bareNixos [
        hardware
        self.nixosModules.bluetooth-disk-prompt
      ];
      firmware = "${machine.config.hardware.firmware}/lib/firmware";
      named = machine.config.boot.initrd.extraFirmwarePaths;
    in
    pkgs.runCommandLocal name { } ''
      missing=0
      for path in ${lib.escapeShellArgs named}; do
        if [ ! -e ${firmware}/$path ] && [ ! -e ${firmware}/$path.zst ] \
          && [ ! -e ${firmware}/$path.xz ]; then
          echo "the disk prompt's firmware $path isn't in the machine's firmware"
          missing=1
        fi
      done
      [ ${toString (lib.length named)} -gt 0 ] || {
        echo "the hardware names no firmware for the disk prompt"
        missing=1
      }
      [ $missing = 0 ] && touch $out
    '';
in
onLinux {
  # The seal, against a made-up BlueZ state and a stand-in systemd-creds,
  # leaves nothing on the ESP with nothing paired, seals a paired keyboard's
  # keys and service records to the TPM under PCR 7 and no device only
  # seen in a scan, seals unchanged pairings only once and changed ones
  # again, and takes the keys off the ESP once unpaired.
  bluetooth-seal-keeps-only-what-a-keyboard-needs =
    pkgs.runCommandLocal "bluetooth-seal-keeps-only-what-a-keyboard-needs"
      {
        nativeBuildInputs = [
          pkgs.coreutils
          pkgs.findutils
          pkgs.gnugrep
          pkgs.gnutar
        ];
      }
      ''
        ${lib.getExe pkgs.bash} ${./seal-test.sh} ${./seal.sh}
        touch $out
      '';

  # Each laptop's Bluetooth controller has its firmware at the disk prompt.
  bluetooth-firmware-reaches-the-disk-prompt-dell-xps-14-da14260 = firmwareReaches "bluetooth-firmware-reaches-the-disk-prompt-dell-xps-14-da14260" self.nixosModules.dell-xps-14-da14260;
  bluetooth-firmware-reaches-the-disk-prompt-microsoft-surface-laptop-4-1958 = firmwareReaches "bluetooth-firmware-reaches-the-disk-prompt-microsoft-surface-laptop-4-1958" self.nixosModules.microsoft-surface-laptop-4-1958;
}
