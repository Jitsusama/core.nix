# The checks of microsoft-surface-laptop-4-1958, which checks.nix gathers with every other folder's;
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
  # The Surface Laptop 4's keyboard is behind Surface's embedded controller,
  # so its disk can be unlocked from it only when the initrd has every driver
  # on the way there, and keyd gives that keyboard Colemak there too.
  the-surface-keyboard-types-at-the-disk-prompt =
    let
      surface = bareNixos [
        self.nixosModules.laptop
        self.nixosModules.microsoft-surface-laptop-4-1958
      ];
      inherit (surface.config) boot environment;
      path = [
        "8250_dw"
        "surface_aggregator"
        "surface_aggregator_registry"
        "surface_aggregator_hub"
        "surface_hid_core"
        "surface_hid"
      ];
      keyd = environment.etc."keyd/built-in.conf".text or "";
    in
    holds "the-surface-keyboard-types-at-the-disk-prompt"
      "The Surface Laptop 4's initrd lacks a driver on its keyboard's path, or keyd doesn't lay that keyboard out."
      (
        lib.all (module: lib.elem module boot.initrd.availableKernelModules) path
        && lib.hasInfix "045e:09ae" keyd
        && boot.initrd.systemd.contents ? "/etc/keyd/built-in.conf"
      );

  # The GPU's driver loads in the initrd, before the console is set up, and
  # the firmware's unreadable clock never becomes rtc0, and the CMOS clock
  # sets the time in the initrd, before systemd moves it forward.
  the-surface-boots-with-its-gpu-and-a-clock-that-reads =
    let
      surface = bareNixos [
        self.nixosModules.laptop
        self.nixosModules.microsoft-surface-laptop-4-1958
      ];
      inherit (surface.config) boot;
    in
    holds "the-surface-boots-with-its-gpu-and-a-clock-that-reads"
      "The Surface Laptop 4 loads amdgpu or rtc_cmos only after its initrd, or lets acpi_tad's unreadable clock take rtc0."
      (
        lib.elem "amdgpu" boot.initrd.kernelModules
        && lib.elem "rtc_cmos" boot.initrd.kernelModules
        && lib.elem "acpi_tad" boot.blacklistedKernelModules
      );

  # A sleep stops the chipset's watchdog counting, so the hook stops
  # systemd's runtime watchdog before each sleep and starts it after, at the
  # timeout systemd ran with. Run against a stand-in of busctl, a sleep with
  # the watchdog running stops and restarts it, and one without it touches
  # nothing.
  the-surface-watchdog-wakes-again =
    let
      surface = bareNixos [
        self.nixosModules.laptop
        self.nixosModules.microsoft-surface-laptop-4-1958
      ];
      inherit (surface.config) environment systemd;
      hook = environment.etc."systemd/system-sleep/watchdog-wakes-again".source;
    in
    pkgs.runCommandLocal "the-surface-watchdog-wakes-again" { } ''
      export BUSCTL=$PWD/busctl KEPT=$PWD/kept
      fail() {
        echo "$1" >&2
        exit 1
      }
      [ "${systemd.settings.Manager.RuntimeWatchdogSec}" = 30s ] ||
        fail "The Surface Laptop 4 doesn't arm the runtime watchdog at 30 s."

      # The stand-in answers with the timeout in ./running and logs each change.
      {
        printf '#!%s\n' "$BASH"
        echo 'case "$1" in'
        echo 'get-property) echo "t $(cat "$(dirname "$0")/running")" ;;'
        echo 'set-property) echo "$7" >>"$(dirname "$0")/set" ;;'
        echo 'esac'
      } >busctl
      chmod +x busctl

      echo 30000000 >running
      ${hook} pre suspend
      [ "$(cat set)" = 0 ] || fail "A sleep left the runtime watchdog running."
      ${hook} post suspend
      [ "$(cat set)" = "$(printf '0\n30000000')" ] ||
        fail "The runtime watchdog wasn't started again at its timeout after a sleep."
      [ -e kept ] && fail "The kept timeout outlived the sleep."

      rm set
      echo 0 >running
      ${hook} pre hibernate
      ${hook} post hibernate
      [ -e set ] && fail "A sleep started a runtime watchdog that wasn't running."
      touch $out
    '';
}
