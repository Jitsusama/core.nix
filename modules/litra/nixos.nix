# Joel's Logitech Litra Glow, the light on his monitor: the `litra` command
# that turns it on and off, sets its brightness and warmth, and says what it's
# doing, once or as it changes (`litra watch`, for the shell), and a watch,
# run as the person at the machine, that lights it while any camera streams
# and puts it out once none does, so a call is lit without a hand to the
# button. It keeps the light out while the screen is locked, as it is before
# the machine sleeps, by the file Quickshell's lock writes. systemd's hwdb
# opens the light's hidraw node to the person at the seat, so nothing else
# needs to.
#
# Which cameras stream it reads from /run/camera-holders, as the shell's
# camera piece does, so the light and the piece always agree: the loopback
# cameras' files (../v4l2loopback) and the USB cameras' (../usb-camera-holders).
# A light plugged in while a camera streams is lit as udev opens it. The
# watch sleeps on inotify between changes, so it costs nothing while
# nothing changes; litra.rs says how.
{ pkgs, ... }:
let
  # The standard library alone, so it builds with rustc and no crates.
  litra = pkgs.runCommandCC "litra" { nativeBuildInputs = [ pkgs.rustc ]; } ''
    mkdir -p $out/bin
    rustc --edition 2024 -C opt-level=3 -D warnings -o $out/bin/litra ${./litra.rs}
  '';
in
{
  imports = [ ../usb-camera-holders/nixos.nix ];

  environment.systemPackages = [ litra ];

  systemd.user.services.litra-follows-cameras = {
    description = "Lights the Litra Glow while a camera streams";
    wantedBy = [ "default.target" ];
    serviceConfig = {
      ExecStart = "${litra}/bin/litra follow /run/camera-holders %t/quickshell-locked";
      Restart = "on-failure";
      RestartSec = 5;
      NoNewPrivileges = true;
      PrivateNetwork = true;
      RestrictAddressFamilies = "none";
      ProtectSystem = "strict";
      ProtectHome = "read-only";
      PrivateTmp = true;
      CapabilityBoundingSet = "";
      RestrictNamespaces = true;
      RestrictRealtime = true;
      LockPersonality = true;
      MemoryDenyWriteExecute = true;
      SystemCallArchitectures = "native";
      SystemCallFilter = [
        "@system-service"
        "~@privileged @resources"
      ];
    };
  };
}
