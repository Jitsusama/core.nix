# How strongly a haptic touchpad clicks, when a machine says. A haptic pad
# has no button: its firmware buzzes when pressed, at an intensity its report
# descriptor offers as a setting, and starts at its own default,
# which no driver on Linux changes. click-strength.rs finds the intensity in
# each pad's report descriptor and sets it when the pad appears, and again
# after every sleep in case the firmware forgets. Unset, nothing runs.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.touchpad) clickStrength;

  # The standard library alone, so it builds with rustc and no crates.
  setter = pkgs.runCommandCC "touchpad-click-strength" { nativeBuildInputs = [ pkgs.rustc ]; } ''
    mkdir -p $out/bin
    rustc --edition 2024 -C opt-level=3 -D warnings \
      -o $out/bin/touchpad-click-strength ${./click-strength.rs}
  '';
  strength = toString clickStrength;

  sleeps = [
    "suspend.target"
    "hibernate.target"
    "hybrid-sleep.target"
    "suspend-then-hibernate.target"
  ];
in
{
  options.jitsusama.touchpad.clickStrength = lib.mkOption {
    type = lib.types.nullOr (lib.types.ints.between 0 100);
    default = null;
    example = 30;
    description = ''
      How strongly a haptic touchpad clicks, as a percentage of what its
      firmware allows. Null leaves the firmware's own. Devices without a haptic
      intensity are passed over.
    '';
  };

  config = lib.mkIf (clickStrength != null) {
    services.udev.extraRules = ''
      ACTION=="add", SUBSYSTEM=="hidraw", RUN+="${setter}/bin/touchpad-click-strength ${strength} %k"
    '';

    # After sleep, a unit wanted by each sleep target runs once the machine
    # is back, as the target is only reached when it wakes.
    systemd.services.touchpad-click-strength = {
      description = "Set how strongly the touchpad clicks, after sleep";
      after = sleeps;
      wantedBy = sleeps;
      serviceConfig = {
        Type = "oneshot";
        ExecStart = "${setter}/bin/touchpad-click-strength ${strength}";
        PrivateNetwork = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        NoNewPrivileges = true;
        RestrictAddressFamilies = "none";
        CapabilityBoundingSet = "";
        SystemCallArchitectures = "native";
      };
    };
  };
}
