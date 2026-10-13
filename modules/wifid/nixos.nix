# wifid joins Wi-Fi networks through wpa_supplicant and gives networkd a file
# for each one the launcher saves. It runs as root only to reach
# wpa_supplicant and networkd, with no capabilities, and writes nowhere but
# its own state and networkd's runtime files.
{ lib, pkgs, ... }:
let
  wifid = pkgs.callPackage ./package.nix { };
in
{
  imports = [
    ../systemd-networkd/nixos.nix
    ../wpa_supplicant/nixos.nix
  ];

  systemd.services.wifid = {
    description = "Wi-Fi networks, joined and checked for a sign-in";
    wants = [ "wpa_supplicant.service" ];
    after = [
      "wpa_supplicant.service"
      "systemd-networkd.service"
    ];
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      ExecStart = lib.getExe wifid;
      Restart = "on-failure";
      StateDirectory = "wifid";
      StateDirectoryMode = "0700";
      UMask = "0077";
      # networkd reads what wifid writes here, so it must exist before the
      # filesystem is made read-only around wifid.
      ReadWritePaths = [ "/run/systemd/network" ];
      CapabilityBoundingSet = "";
      NoNewPrivileges = true;
      LockPersonality = true;
      MemoryDenyWriteExecute = true;
      PrivateDevices = true;
      PrivateTmp = true;
      ProtectClock = true;
      ProtectControlGroups = true;
      ProtectHome = true;
      ProtectHostname = true;
      ProtectKernelLogs = true;
      ProtectKernelModules = true;
      ProtectKernelTunables = true;
      ProtectSystem = "strict";
      RestrictAddressFamilies = [
        "AF_UNIX"
        "AF_INET"
        "AF_INET6"
      ];
      RestrictNamespaces = true;
      RestrictRealtime = true;
      SystemCallArchitectures = "native";
      SystemCallFilter = [ "@system-service" ];
    };
  };

  systemd.tmpfiles.rules = [ "d /run/systemd/network 0755 root root -" ];
}
