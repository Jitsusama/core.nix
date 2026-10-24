# What the launcher shows behind each display's and USB sound card's row,
# and in its camera answer, that it can't learn for itself: a display's EDID
# (who made it and when, its size, native mode, refresh range, colour depth,
# colours against sRGB, HDR); a card's formats, clock, USB speed, firmware
# and, while it plays or records, what it runs at; and a USB camera's
# formats, field of view, USB speed, firmware and what it streams.
# desk-facts, run as the person at the machine, keeps them as JSON in
# $XDG_RUNTIME_DIR/desk, one file a kind, rewriting a file only when a
# uevent, a sound device opening or closing or a camera's holders changing
# alters what it says; facts.rs says how. The Pebble X Plus's lighting it
# reads from what pebble-lights writes (../creative-pebble-x-plus), and who
# streams from /run/camera-holders (../usb-camera-holders).
{ pkgs, ... }:
let
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
      ExecStart = "${facts}/bin/desk-facts %t/desk";
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
