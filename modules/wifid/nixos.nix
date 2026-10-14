# wifid joins Wi-Fi networks through wpa_supplicant and gives networkd a file
# for each one the launcher saves, and checks each network it reaches with
# the pages wifid.conf names. It runs as root only to reach wpa_supplicant
# and networkd, with no capabilities, and writes nowhere but its own state
# and networkd's runtime files.
#
# It answers over varlink, as io.jitsusama.Wifi, on a socket systemd holds
# for it, which the shell and varlinkctl talk to. Every account home-manager
# keeps is in its group, wifi, and nobody else can reach it, since through
# it a network can be joined, trusted or forgotten.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  wifid = pkgs.callPackage ./package.nix { };
in
{
  imports = [
    ../systemd-networkd/nixos.nix
    ../wpa_supplicant/nixos.nix
  ];

  environment.etc."wifid/wifid.conf".source = ./wifid.conf;

  users.groups.wifi = { };
  users.users = lib.mapAttrs (_: _: {
    extraGroups = [ "wifi" ];
  }) config.home-manager.users;
  # Each of them has the sign-in window too.
  home-manager.sharedModules = [ ./home.nix ];

  # systemd keeps the socket while wifid starts again, so whoever's talking
  # to it waits a moment rather than finding nothing there.
  systemd.sockets.wifid = {
    description = "Where the shell talks to wifid";
    wantedBy = [ "sockets.target" ];
    listenStreams = [ "/run/wifid/io.jitsusama.Wifi" ];
    socketConfig = {
      # Where varlinkctl list-registry finds it, beside systemd's own.
      Symlinks = [ "/run/varlink/registry/io.jitsusama.Wifi" ];
      FileDescriptorName = "varlink";
      SocketGroup = "wifi";
      SocketMode = "0660";
      DirectoryMode = "0755";
    };
  };

  systemd.services.wifid = {
    description = "Wi-Fi networks, joined and checked for a sign-in";
    # It reads wifid.conf once, as it starts.
    restartTriggers = [ config.environment.etc."wifid/wifid.conf".source ];
    wants = [ "wpa_supplicant.service" ];
    requires = [ "wifid.socket" ];
    after = [
      "wifid.socket"
      "wpa_supplicant.service"
      "systemd-networkd.service"
    ];
    wantedBy = [ "multi-user.target" ];
    # Without wifid no card joins anything, so it never stays down: it
    # comes back however it ended, the bus going away included, waiting a
    # little longer each time up to a minute, and is never given up on.
    # Starting again leaves every card on its network.
    startLimitIntervalSec = 0;
    serviceConfig = {
      ExecStart = lib.getExe wifid;
      Restart = "always";
      RestartSec = "1s";
      RestartSteps = 6;
      RestartMaxDelaySec = "1min";
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
      # The radios' switches are the one device wifid needs, to say whether
      # Wi-Fi is on and turn it off and on for the shell. A machine without
      # them still starts wifid, which then shows Wi-Fi on.
      PrivateDevices = true;
      BindPaths = [ "-/dev/rfkill" ];
      DeviceAllow = [ "/dev/rfkill rw" ];
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

  # The boot waits for the network only while there's one to wait for:
  # wifid answers Settled once a card or cable reaches beyond the machine,
  # Wi-Fi is off, or every card has looked and heard no saved network it
  # can join. networkd's own wait can't tell a card still joining from one
  # with nothing to join, so it's turned off. Thirty seconds is a backstop.
  systemd.network.wait-online.enable = false;
  systemd.services.wifid-online = {
    description = "Wait for Wi-Fi to join what it can";
    wantedBy = [ "network-online.target" ];
    before = [
      "network-online.target"
      "shutdown.target"
    ];
    requires = [ "wifid.socket" ];
    after = [ "wifid.socket" ];
    conflicts = [ "shutdown.target" ];
    unitConfig.DefaultDependencies = false;
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
      ExecStart = lib.escapeShellArgs [
        "${config.systemd.package}/bin/varlinkctl"
        "call"
        "--timeout=30"
        "/run/wifid/io.jitsusama.Wifi"
        "io.jitsusama.Wifi.Settled"
        "{}"
      ];
    };
  };

  systemd.tmpfiles.rules = [ "d /run/systemd/network 0755 root root -" ];
}
