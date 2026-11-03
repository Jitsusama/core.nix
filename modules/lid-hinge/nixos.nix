# The lid, shut only when the hinge agrees. A lid's switch can say shut
# with the lid wide open, from pressure on the chassis, and on battery
# logind would put the laptop to sleep under the hands typing on it.
# lid-hinge.rs offers logind and the compositor a switch of its own, which
# says shut only when the hinge sensor saw the lid coming down, or a light
# sensor went dark; its opening comment says how it tells. udev hides the
# lid's own switch from both and turns on the hinge's buffer, which the
# program wakes for only when the angle changes. Without the program
# there's no lid at all, so it's started again at once if it stops.
#
# udev hides a switch only as it appears, so a switch that adds this
# leaves logind and the compositor watching the lid's own switch, beside
# the program's, until the next boot.
{ lib, pkgs, ... }:
let
  # The standard library alone, so it builds with rustc and no crates.
  lidHinge = pkgs.runCommandCC "lid-hinge" { nativeBuildInputs = [ pkgs.rustc ]; } ''
    mkdir -p $out/bin
    rustc --edition 2024 -C opt-level=3 -D warnings \
      -o $out/bin/lid-hinge ${./lid-hinge.rs}
  '';
in
{
  # industrialio from the start, before the sensor hub's drivers bring it:
  # systemd resolves DeviceAllow's char-iio from /proc/devices once, as
  # the unit starts, so one started before iio's devices are registered
  # can never open the hinge. systemd-modules-load loads these before any
  # service starts.
  boot.kernelModules = [
    "industrialio"
    "uinput"
  ];

  # After 60-input-id.rules, which says a switch is one, and before
  # 70-power-switch.rules, which tags it for logind.
  services.udev.packages = [
    (pkgs.writeTextDir "lib/udev/rules.d/65-lid-hinge.rules" (builtins.readFile ./lid-hinge.rules))
  ];

  # Root's, for uinput and the hinge's buffer.
  systemd.services.lid-hinge = {
    description = "Offers a lid that shuts only when the hinge agrees";
    wantedBy = [ "multi-user.target" ];
    # Until the lid's switch appears it stops at once, so it's tried again
    # every second for as long as that takes.
    startLimitIntervalSec = 0;
    serviceConfig = {
      Type = "exec";
      ExecStart = lib.getExe' lidHinge "lid-hinge";
      Restart = "always";
      RestartSec = 1;
      CapabilityBoundingSet = "";
      DeviceAllow = [
        "/dev/uinput rw"
        "char-input r"
        "char-iio r"
      ];
      DevicePolicy = "closed";
      LockPersonality = true;
      MemoryDenyWriteExecute = true;
      NoNewPrivileges = true;
      PrivateNetwork = true;
      PrivateTmp = true;
      ProtectHome = true;
      ProtectKernelTunables = true;
      ProtectSystem = "strict";
      RestrictAddressFamilies = "AF_UNIX";
      RestrictNamespaces = true;
      RestrictRealtime = true;
      SystemCallArchitectures = "native";
      SystemCallFilter = [
        "@system-service"
        "~@privileged"
      ];
    };
  };
}
