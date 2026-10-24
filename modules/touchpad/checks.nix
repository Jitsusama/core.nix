# The checks of touchpad, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  lib,
  onLinux,
  pkgs,
  self,
  ...
}:
{
  the-click-strength-holds-after-sleep =
    let
      machine =
        strength:
        (bareNixos [
          self.nixosModules.touchpad
          { jitsusama.touchpad.clickStrength = strength; }
        ]).config;
      set = machine 30;
      unset = machine null;
      unit = set.systemd.services.touchpad-click-strength or { };
      sleeps = [
        "suspend.target"
        "hibernate.target"
        "hybrid-sleep.target"
        "suspend-then-hibernate.target"
      ];
    in
    holds "the-click-strength-holds-after-sleep"
      "The touchpad isn't set to 30% as it appears and after each sleep, or it's set unasked."
      (
        lib.hasInfix "touchpad-click-strength 30 %k" set.services.udev.extraRules
        && lib.hasSuffix "touchpad-click-strength 30" (unit.serviceConfig.ExecStart or "")
        && lib.all (target: lib.elem target unit.wantedBy && lib.elem target unit.after) sleeps
        && !(unset.systemd.services ? touchpad-click-strength)
        && !(lib.hasInfix "touchpad-click-strength" unset.services.udev.extraRules)
      );
}
// onLinux {
  # The click strength's setter finds the intensity in a haptic touchpad's
  # report descriptor, the XPS 14's among them, passes over a device
  # without one, and writes only that field, only when it differs.
  the-click-strength-finds-the-intensity =
    pkgs.runCommandCC "the-click-strength-finds-the-intensity" { nativeBuildInputs = [ pkgs.rustc ]; }
      ''
        rustc --edition 2024 --test -D warnings -o tests ${./click-strength.rs}
        ./tests
        touch $out
      '';
}
