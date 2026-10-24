# How each connected Bluetooth link is doing, for the launcher's Bluetooth
# to say beside a device: its signal, how well it carries, and which of the
# radio's 79 channels it skips as noisy. BlueZ says none of it for a
# connected device, so bluetooth-links asks the controller every five
# seconds while a link is up, and only lists the connections while none is
# (src/main.rs says how). It writes /run/bluetooth-links/links.json,
# readable by everyone, which /run/bluetooth-links.json points at, so the
# file goes when the reader does rather than saying how things stood.
{ lib, pkgs, ... }:
let
  links = pkgs.callPackage ./package.nix { };
in
{
  systemd.services.bluetooth-links = {
    description = "Says how each Bluetooth link is doing, for the launcher";
    wantedBy = [ "multi-user.target" ];
    after = [ "bluetooth.service" ];
    serviceConfig = {
      ExecStart = "${lib.getExe links} %t/bluetooth-links/links.json";
      Restart = "on-failure";
      RestartSec = 5;
      # It writes in a folder of its own, renaming each file into place so
      # a reader never sees one half written.
      RuntimeDirectory = "bluetooth-links";
      RuntimeDirectoryMode = "0755";
      UMask = "0022";
      # It wakes for a few milliseconds every five seconds, and nothing
      # waits on it, so it yields to everything.
      Nice = 19;
      CPUWeight = 10;
      IOWeight = 10;
      # The kernel lets a socket with no capability list the connections
      # and send the four commands asked (hci_sec_filter in
      # net/bluetooth/hci_sock.c), which an account with none was seen to
      # do, so it runs as nobody in particular, holding none.
      DynamicUser = true;
      CapabilityBoundingSet = "";
      AmbientCapabilities = "";
      NoNewPrivileges = true;
      # Bluetooth's sockets are made only in the machine's own network
      # namespace, so it keeps that one, and may open no other kind.
      RestrictAddressFamilies = "AF_BLUETOOTH";
      IPAddressDeny = "any";
      PrivateDevices = true;
      PrivateTmp = true;
      PrivateUsers = true;
      ProtectSystem = "strict";
      ProtectHome = true;
      ProtectKernelTunables = true;
      ProtectKernelModules = true;
      ProtectKernelLogs = true;
      ProtectControlGroups = true;
      ProtectClock = true;
      ProtectHostname = true;
      ProtectProc = "invisible";
      ProcSubset = "pid";
      RestrictNamespaces = true;
      RestrictRealtime = true;
      RestrictSUIDSGID = true;
      LockPersonality = true;
      MemoryDenyWriteExecute = true;
      SystemCallArchitectures = "native";
      SystemCallFilter = [
        "@system-service"
        "~@privileged @resources"
      ];
    };
  };

  systemd.tmpfiles.settings.bluetooth-links."/run/bluetooth-links.json"."L+".argument =
    "/run/bluetooth-links/links.json";
}
