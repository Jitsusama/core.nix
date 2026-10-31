# The checks of microsoft-surface-laptop-4-1958, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  lib,
  passesClippy,
  pkgs,
  self,
  ...
}:
{
  # clippy finds nothing in surface-clock, its tests included.
  surface-clock-passes-clippy = passesClippy "surface-clock-passes-clippy" (
    pkgs.callPackage ./clock/package.nix { }
  );

  # The controller's clock is read for the firmware's offset before timesyncd
  # can have the kernel write the CMOS clock, through the device its module
  # gives, and set at shutdown and every hour.
  the-surface-keeps-the-clock-its-firmware-starts-from =
    let
      surface = bareNixos [
        self.nixosModules.laptop
        self.nixosModules.microsoft-surface-laptop-4-1958
      ];
      inherit (surface.config) boot systemd;
      clock = systemd.services.surface-clock;
    in
    holds "the-surface-keeps-the-clock-its-firmware-starts-from"
      "The Surface Laptop 4 reads its controller's clock after timesyncd may have written the CMOS clock, lacks the controller's device, or doesn't set the controller's clock at shutdown and hourly."
      (
        lib.elem "surface_aggregator_cdev" boot.kernelModules
        && lib.elem "systemd-timesyncd.service" clock.before
        && lib.elem "dev-surface-aggregator.device" clock.after
        && lib.hasSuffix "calibrate" clock.serviceConfig.ExecStart
        && lib.hasSuffix "keep" clock.serviceConfig.ExecStop
        && systemd.timers.surface-clock-keep.timerConfig.OnUnitActiveSec == "1h"
      );

  # The touchscreen's driver is installed with the panel's qualified settings,
  # and loaded by a unit once the system is up, never while it starts.
  the-surface-loads-its-touchscreen-once-up =
    let
      surface = bareNixos [
        self.nixosModules.laptop
        self.nixosModules.microsoft-surface-laptop-4-1958
      ];
      inherit (surface.config) boot systemd;
      touchscreen = systemd.services.surface-touchscreen;
    in
    holds "the-surface-loads-its-touchscreen-once-up"
      "The Surface Laptop 4 lacks its touchscreen's driver or its panel's settings, or loads the driver before the system is up."
      (
        lib.any (package: lib.hasPrefix "sl4a-touchscreen-" package.name) boot.extraModulePackages
        && lib.hasInfix "options sl4a_spi_hid raw_mode=N" boot.extraModprobeConfig
        && !(lib.elem "sl4a_spi_amd" boot.kernelModules)
        && !(lib.elem "sl4a_spi_hid" boot.kernelModules)
        && lib.elem "multi-user.target" touchscreen.after
        && lib.hasSuffix "sl4a_spi_amd sl4a_spi_hid" touchscreen.serviceConfig.ExecStart
      );

  # fwupd, run against this model's ESRT with the UEFI and the TPM behind,
  # offers the UEFI's update from the remote the module writes and nothing
  # for the TPM (firmware/tests/offers-its-firmware.sh). Building the remote's
  # cabinet checks the capsule is the UEFI's at the version it's offered as.
  the-surface-offers-its-firmware-through-fwupd =
    let
      surface = bareNixos [
        self.nixosModules.laptop
        self.nixosModules.microsoft-surface-laptop-4-1958
      ];
      inherit (surface.config) environment services;
      remote =
        pkgs.writeText "surface-laptop-4-amd.conf"
          environment.etc."fwupd/remotes.d/surface-laptop-4-amd.conf".text;
    in
    pkgs.runCommandLocal "the-surface-offers-its-firmware-through-fwupd" { } ''
      ${lib.getExe pkgs.bash} ${./firmware/tests/offers-its-firmware.sh} ${remote} \
        ${services.fwupd.package} ${lib.getExe pkgs.jq}
      touch $out
    '';

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
  # nothing. The hook runs with an empty PATH, since systemd-sleep's has no
  # coreutils and the hook's after half once failed there on cat. The
  # stand-in needs cat and dirname too, so it finds them only through the
  # PATH the hook makes.
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

      hook() { PATH=/var/empty ${hook} "$@"; }

      echo 30000000 >running
      hook pre suspend
      [ "$(cat set)" = 0 ] || fail "A sleep left the runtime watchdog running."
      hook post suspend
      [ "$(cat set)" = "$(printf '0\n30000000')" ] ||
        fail "The runtime watchdog wasn't started again at its timeout after a sleep."
      [ -e kept ] && fail "The kept timeout outlived the sleep."

      rm set
      echo 0 >running
      hook pre hibernate
      hook post hibernate
      [ -e set ] && fail "A sleep started a runtime watchdog that wasn't running."
      touch $out
    '';
}
