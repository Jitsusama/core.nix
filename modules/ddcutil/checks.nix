# The checks of ddcutil, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  lib,
  onLinux,
  pkgs,
  self,
  ...
}:
onLinux {
  # The seat gets the buses that reach external DisplayPort monitors, a
  # connector's AUX channel and an MST branch's, and no other: not the
  # panel's eDP, not HDMI's, not the touchpad's. A VM has no DisplayPort,
  # so each rule's patterns are matched, as udev matches them, against the
  # buses of a laptop with a monitor behind a USB-C hub: each bus's own
  # name and the device names from it up to the PCI root. The rules must
  # also come before systemd's 73-seat-late.rules, which reads the tag
  # (tests/only-external-monitors-reach-the-seat.sh).
  only-external-monitors-reach-the-seat =
    let
      machine = bareNixos [ self.nixosModules.ddcutil ];
      file = "lib/udev/rules.d/70-monitor-controls.rules";
      rules = lib.findFirst (
        each: (each.name or "") == baseNameOf file
      ) (throw "no rules give monitors' buses to the seat") machine.config.services.udev.packages;
      loaded = lib.elem "i2c-dev" machine.config.boot.kernelModules;
    in
    pkgs.runCommandLocal "only-external-monitors-reach-the-seat" { } ''
      ${lib.optionalString (!loaded) ''
        echo "i2c-dev isn't loaded, so no bus is offered"
        exit 1
      ''}
      ${lib.getExe pkgs.bash} ${./tests/only-external-monitors-reach-the-seat.sh} ${rules}/${file}
      touch $out
    '';
}
