# wpa_supplicant does the radio's half of Wi-Fi: scanning, joining with WPA2,
# WPA3 or an enterprise login, and moving between access points. It holds no
# networks of its own. wifid adds each Wi-Fi card over D-Bus, as it appears,
# with the settings below, and hands it the networks to join.
#
# NixOS's own wpa_supplicant module isn't used. Given no card's name, it
# restarts wpa_supplicant whenever any card comes or goes, which takes every
# other card off its network; a USB card plugged in would drop the laptop's
# own. This runs wpa_supplicant as its upstream unit does, started by D-Bus,
# with nothing restarting it but a change to it.
#
# Each card stays awake on mains power and dozes on battery (power-save.sh).
{
  config,
  lib,
  pkgs,
  ...
}:
let
  # After an access point doesn't answer, upstream's wpa_supplicant can pick
  # that same one again while still marked as joining it, do nothing, and
  # then wait with no look asked for. The patch's header says how.
  wpa_supplicant = pkgs.wpa_supplicant.overrideAttrs (old: {
    patches = (old.patches or [ ]) ++ [ ./retry-after-a-failed-join.patch ];
  });

  # Whether each card dozes, by the power it runs on (power-save.sh says why).
  powerSave = pkgs.writeShellApplication {
    name = "wifi-power-save";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.iw
      pkgs.util-linux
    ];
    text = builtins.readFile ./power-save.sh;
  };
in
{
  # What every card wifid adds starts from. wpa_cli reaches a card through
  # its control socket, for root and the wpa_supplicant group, since it can
  # change any network. Nothing is saved back: wifid owns the networks.
  environment.etc."wpa_supplicant/interface.conf".text = ''
    ctrl_interface=DIR=/run/wpa_supplicant/control GROUP=wpa_supplicant
    update_config=0
    # A card asks for network details from a random address, and joins each
    # network from the one wifid gives it. wifid has the driver look for
    # networks from random addresses too, which, unlike preassoc_mac_addr,
    # leaves the card's own address and so its link alone.
    gas_rand_mac_addr=1
    # No Wi-Fi Direct: it would add a second interface beside every card.
    p2p_disabled=1
    # Looks for a better access point of the same network when the signal
    # drops below -70 dBm, every 30 seconds, and hourly otherwise.
    bgscan="simple:30:-70:3600"
    # Protects management frames wherever the access point can, so nobody
    # nearby can knock the card off a network that offers it.
    pmf=1
    # Turns a WPA3 password into a key either way an access point asks for,
    # the newer hash-to-element way included, which some offer alone.
    sae_pwe=2
  '';

  # udev sets each Wi-Fi card as it appears, and every card again when a
  # charger comes or goes, which a battery's own changes as it drains don't.
  services.udev.extraRules = ''
    SUBSYSTEM=="net", ACTION=="add", ENV{DEVTYPE}=="wlan", RUN+="${lib.getExe powerSave}"
    SUBSYSTEM=="power_supply", ATTR{type}=="Mains|USB", RUN+="${lib.getExe powerSave}"
  '';
  # A switch reloads those rules without running them, so this sets every
  # card once more as the boot reaches its services, and whenever a switch
  # brings a different script.
  systemd.services.wifi-power-save = {
    description = "Keep Wi-Fi awake on mains power";
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
      ExecStart = lib.getExe powerSave;
    };
  };

  users.users.wpa_supplicant = {
    isSystemUser = true;
    group = "wpa_supplicant";
    description = "wpa_supplicant";
  };
  users.groups.wpa_supplicant = { };

  # Its D-Bus policy, and the file that has D-Bus start it as the unit below.
  services.dbus.packages = [ wpa_supplicant ];
  environment.systemPackages = [
    wpa_supplicant
    pkgs.iw
  ];

  systemd.services.wpa_supplicant = {
    description = "Wi-Fi's radio half, for the cards wifid gives it";
    before = [ "network.target" ];
    wants = [ "network.target" ];
    after = [ "dbus.service" ];
    # A switch restarts it only when it or the settings above change, which
    # only a card given to it anew reads, and without stopping it first, so
    # the cards are off their networks for as short a time as can be.
    stopIfChanged = false;
    restartTriggers = [ config.environment.etc."wpa_supplicant/interface.conf".source ];
    serviceConfig = {
      Type = "dbus";
      BusName = "fi.w1.wpa_supplicant1";
      ExecStart = "${lib.getExe' wpa_supplicant "wpa_supplicant"} -u -s";
      User = "wpa_supplicant";
      Group = "wpa_supplicant";
      # Each card's control socket, and wpa_cli's own beside it, for root
      # and the wpa_supplicant group.
      RuntimeDirectory = [
        "wpa_supplicant"
        "wpa_supplicant/client"
      ];
      RuntimeDirectoryMode = "0770";
      AmbientCapabilities = [
        "CAP_NET_ADMIN"
        "CAP_NET_RAW"
      ];
      CapabilityBoundingSet = [
        "CAP_NET_ADMIN"
        "CAP_NET_RAW"
      ];
      # It follows the radio switch through rfkill, and sets a few of the
      # kernel's network settings for a card on some networks.
      DevicePolicy = "closed";
      DeviceAllow = [ "/dev/rfkill rw" ];
      ReadWritePaths = [ "/proc/sys/net" ];
      IPAddressDeny = "any";
      LockPersonality = true;
      MemoryDenyWriteExecute = true;
      NoNewPrivileges = true;
      PrivateTmp = true;
      ProtectClock = true;
      ProtectControlGroups = true;
      ProtectHome = true;
      ProtectHostname = true;
      ProtectKernelLogs = true;
      ProtectKernelModules = true;
      ProtectKernelTunables = true;
      ProtectProc = "invisible";
      ProtectSystem = "strict";
      RemoveIPC = true;
      RestrictAddressFamilies = [
        "AF_UNIX"
        "AF_INET"
        "AF_INET6"
        "AF_NETLINK"
        "AF_PACKET"
      ];
      RestrictNamespaces = true;
      RestrictRealtime = true;
      RestrictSUIDSGID = true;
      SystemCallArchitectures = "native";
      SystemCallFilter = [
        "@system-service"
        "~@keyring"
        "~@resources"
      ];
      UMask = "0077";
    };
  };
}
