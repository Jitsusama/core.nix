# What the launcher shows behind each display's and sound device's row,
# and in its camera answer, that it can't learn for itself: a display's EDID
# (who made it and when, its size, native mode, refresh range, colour depth,
# colours against sRGB, HDR), its USB-C cable's data mode and power, and
# what a monitor says over DDC/CI, which the backlight daemon keeps beside
# them (../backlight); a USB card's formats, clock, USB speed and firmware,
# a built-in card's jacks and chips, and, while either plays or records,
# what it runs at; a USB camera's formats, field of view, USB speed,
# firmware and what it streams, and a loopback camera's mode, sensor, which
# way it faces and whether it's powered; a Bluetooth input device's link,
# buttons, wheels or keys and ids; and another keyboard's or a USB mouse's
# polling rate, keys or buttons and wheel, and firmware. desk-facts, run as the person at
# the machine, keeps them as JSON in $XDG_RUNTIME_DIR/desk, one file a kind,
# rewriting a file only when a uevent, a sound device opening or closing, a
# jack, a camera's holders changing or a monitor's facts being kept alters
# what it says; facts.rs says how. The Pebble X Plus's lighting it reads
# from what pebble-lights writes (../creative-pebble-x-plus), who streams
# from /run/camera-holders (../usb-camera-holders), and which loopback
# camera is which, and its sensor, from v4l2loopback's holders, where that
# module is imported (../v4l2loopback).
{
  config,
  lib,
  pkgs,
  ...
}:
let
  # Each loopback camera with a name, a line each: its holders file's
  # name, the loopback's name and its sensor, apart by tabs.
  holders = lib.attrByPath [ "jitsusama" "v4l2loopback" "holders" ] { } config;
  loopbacks = pkgs.writeText "desk-facts-loopbacks" (
    lib.concatStrings (
      lib.mapAttrsToList (
        name: camera:
        lib.optionalString (camera.card != null) "${name}\t${camera.card}\t${toString camera.sensor}\n"
      ) holders
    )
  );
  # The standard library alone, so it builds with rustc and no crates.
  facts = pkgs.runCommandCC "desk-facts" { nativeBuildInputs = [ pkgs.rustc ]; } ''
    mkdir -p $out/bin
    rustc --edition 2024 -C opt-level=3 -D warnings -o $out/bin/desk-facts ${./facts.rs}
  '';
in
{
  systemd.user.services.desk-facts = {
    description = "Facts about the desk's displays and sound cards, for the launcher";
    wantedBy = [ "default.target" ];
    serviceConfig = {
      ExecStart = "${facts}/bin/desk-facts %t/desk ${loopbacks}";
      Restart = "on-failure";
      RestartSec = 5;
      NoNewPrivileges = true;
      # The kernel's uevents come over netlink, and only into the machine's
      # own network namespace, so it keeps that one and may open nothing
      # else on it.
      RestrictAddressFamilies = "AF_NETLINK";
      ProtectSystem = "strict";
      ReadWritePaths = [ "%t" ];
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
