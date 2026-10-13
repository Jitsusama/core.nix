# Joins Wi-Fi networks in a virtual machine the way a laptop meets them, with
# networkd and wpa_supplicant as the laptop role brings them. Two stand-in
# access points live in a network namespace of their own, so their traffic
# crosses the stand-in air rather than taking a shortcut through the
# machine. A joined network gets its address and DNS server; a resolver for a
# private domain, set up the way a development DNS server is, keeps
# answering through every join; and a routing rule another program made, as
# a VPN's client does, survives joining, reconfiguring and networkd
# restarting.
{ self, pkgs }:
let
  inherit (pkgs) lib;

  # The access points' side, run inside their namespace.
  inAir = {
    after = [ "air.service" ];
    requires = [ "air.service" ];
    wantedBy = [ "multi-user.target" ];
    serviceConfig.NetworkNamespacePath = "/run/netns/air";
  };

  # Each names its country, as nearly every real access point does. The
  # test runs nixpkgs's kernel, which records the name of a network joined
  # only when it does; core.nix's kernel has a patch for the others.
  accessPoint =
    name: settings:
    pkgs.writeText "${name}.conf" ''
      driver=nl80211
      hw_mode=g
      country_code=CA
      ieee80211d=1
      ${settings}
    '';

  lantern = accessPoint "lantern" ''
    interface=wlan0
    ssid=Lantern
    channel=6
    wpa=2
    wpa_key_mgmt=WPA-PSK
    rsn_pairwise=CCMP
    wpa_passphrase=glowing in the dark
  '';

  inn = accessPoint "inn" ''
    interface=wlan1
    ssid=Roadside Inn
    channel=11
  '';

  # Each network hands out addresses and answers for its own names.
  networks = pkgs.writeText "networks.conf" ''
    bind-dynamic
    dhcp-leasefile=/run/networks.leases
    interface=wlan0
    interface=wlan1
    dhcp-range=wlan0,10.42.0.10,10.42.0.50,1h
    dhcp-range=wlan1,10.43.0.10,10.43.0.50,1h
    address=/router.lantern/10.42.0.1
    address=/front-desk.inn/10.43.0.1
    no-resolv
  '';
in
pkgs.testers.runNixOSTest {
  name = "wifi";

  nodes.machine =
    { pkgs, ... }:
    {
      imports = [
        self.nixosModules.systemd-networkd
        self.nixosModules.wpa_supplicant
      ];

      # Three stand-in radios: two access points and the laptop's card.
      boot.kernelModules = [ "mac80211_hwsim" ];
      boot.extraModprobeConfig = "options mac80211_hwsim radios=3";
      # A VM turns off wpa_supplicant, since it usually has no radio.
      networking.wireless.enable = lib.mkOverride 0 true;

      # The first two radios move to the access points' namespace before
      # anything joins, leaving wlan2 as the laptop's only card.
      systemd.services.air = {
        after = [ "systemd-modules-load.service" ];
        before = [ "wpa_supplicant.service" ];
        wantedBy = [ "multi-user.target" ];
        path = [
          pkgs.coreutils
          pkgs.iproute2
          pkgs.iw
        ];
        serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
        };
        script = ''
          until [ -e /sys/class/net/wlan1/phy80211/name ]; do sleep 0.1; done
          ip netns add air
          for card in wlan0 wlan1; do
            iw phy "$(cat /sys/class/net/$card/phy80211/name)" set netns name air
          done
          ip -n air link set lo up
          ip -n air address add 10.42.0.1/24 dev wlan0
          ip -n air address add 10.43.0.1/24 dev wlan1
        '';
      };
      systemd.services.lantern = inAir // {
        serviceConfig = inAir.serviceConfig // {
          ExecStart = "${lib.getExe' pkgs.hostapd "hostapd"} ${lantern}";
        };
      };
      systemd.services.inn = inAir // {
        serviceConfig = inAir.serviceConfig // {
          ExecStart = "${lib.getExe' pkgs.hostapd "hostapd"} ${inn}";
        };
      };
      systemd.services.networks = inAir // {
        after = inAir.after ++ [
          "lantern.service"
          "inn.service"
        ];
        serviceConfig = inAir.serviceConfig // {
          ExecStart = "${lib.getExe pkgs.dnsmasq} --keep-in-foreground --conf-file=${networks}";
        };
      };

      # A resolver for a private domain, set up the way dev's is: a global
      # server in resolved.conf.d for its domain alone, on a local port.
      environment.etc."systemd/resolved.conf.d/private.conf".text = ''
        [Resolve]
        DNS=127.0.0.1:5553
        Domains=~corp.internal
      '';
      systemd.services.private-dns = {
        wantedBy = [ "multi-user.target" ];
        serviceConfig.ExecStart = lib.concatStringsSep " " [
          (lib.getExe pkgs.dnsmasq)
          "--keep-in-foreground --no-resolv --no-hosts --bind-interfaces"
          "--listen-address=127.0.0.1 --port=5553"
          "--address=/corp.internal/10.99.0.7"
        ];
      };

      # Roadside Inn has a saved network's file of its own, so joining it
      # moves the card onto a different .network file.
      systemd.network.networks."50-wifi-roadside-inn" = {
        matchConfig.SSID = ''"Roadside Inn"'';
        networkConfig = {
          DHCP = "yes";
          LLMNR = "no";
          MulticastDNS = "no";
        };
      };

      environment.systemPackages = [
        pkgs.iw
        pkgs.dig
      ];
    };

  testScript = ''
    import json

    wpa = "busctl --json=short call fi.w1.wpa_supplicant1"

    def path(reply):
        """The object a wpa_supplicant call made, from busctl's reply."""
        return json.loads(reply)["data"][0]

    def joins(ssid, *secret):
        """Joins a network the way wifid does, over wpa_supplicant's D-Bus."""
        settings = ["ssid", "s", f"'{ssid}'", "scan_ssid", "i", "1"]
        settings += ["psk", "s", f"'{secret[0]}'"] if secret else ["key_mgmt", "s", "NONE"]
        machine.succeed(f"{wpa} {card} fi.w1.wpa_supplicant1.Interface RemoveAllNetworks")
        added = machine.succeed(
            f"{wpa} {card} fi.w1.wpa_supplicant1.Interface AddNetwork 'a{{sv}}' "
            f"{len(settings) // 3} {' '.join(settings)}"
        )
        network = path(added)
        machine.succeed(f"{wpa} {card} fi.w1.wpa_supplicant1.Interface SelectNetwork o {network}")

    def holds_address(prefix):
        machine.wait_until_succeeds(
            f"ip -4 -o address show dev wlan2 | grep -q 'inet {prefix}'", timeout=60
        )
        machine.wait_until_succeeds(
            "networkctl status wlan2 | grep -q 'State: routable'", timeout=30
        )

    def answers(name, address):
        machine.wait_until_succeeds(
            f"resolvectl query --legend=no {name} | grep -qF {address}", timeout=30
        )

    def rules_stand():
        for family in ("-4", "-6"):
            machine.succeed(
                f"ip {family} rule show | grep -q 'not from all fwmark 0x100cf lookup 65743'"
            )

    machine.wait_for_unit("multi-user.target")
    machine.wait_for_unit("networks.service")
    machine.wait_for_unit("private-dns.service")
    machine.wait_for_unit("wpa_supplicant.service")

    with subtest("wpa_supplicant starts with no card, and takes one over D-Bus"):
        # Moving the access points' radios makes udev restart wpa_supplicant,
        # which drops any card it had; wifid adds them again when it does.
        machine.succeed("udevadm settle")
        machine.wait_for_unit("wpa_supplicant.service")
        machine.fail("ls /run/wpa_supplicant/control/wlan2")
        created = machine.succeed(
            f"{wpa} /fi/w1/wpa_supplicant1 fi.w1.wpa_supplicant1 CreateInterface 'a{{sv}}' 2 "
            "Ifname s wlan2 ConfigFile s /etc/wpa_supplicant/interface.conf"
        )
        card = path(created)
        machine.succeed("wpa_cli -i wlan2 ping | grep -qx PONG")

    with subtest("a network joined gets an address and its own DNS server"):
        joins("Lantern", "glowing in the dark")
        holds_address("10.42.0.")
        answers("router.lantern", "10.42.0.1")
        # networkd knows the network by name, which a saved network's file
        # is matched on.
        machine.succeed("networkctl status wlan2 | grep -q 'Wi-Fi access point: Lantern '")
        answers("probe.corp.internal", "10.99.0.7")

    with subtest("a stranger's network gets no LLMNR or mDNS"):
        status = machine.succeed("resolvectl status wlan2")
        assert "-LLMNR" in status and "-mDNS" in status, status

    # WARP's own rule, at its own priority, for both families.
    for family in ("-4", "-6"):
        machine.succeed(f"ip {family} rule add not fwmark 0x100cf lookup 65743 priority 32765")

    with subtest("another program's routing rule survives a reconfigure"):
        machine.succeed("networkctl reconfigure wlan2")
        holds_address("10.42.0.")
        rules_stand()

    with subtest("it survives joining a network with a file of its own"):
        joins("Roadside Inn")
        holds_address("10.43.0.")
        machine.wait_until_succeeds(
            "networkctl status wlan2 | grep -q 'Network File: .*50-wifi-roadside-inn'", timeout=30
        )
        rules_stand()
        answers("front-desk.inn", "10.43.0.1")
        answers("probe.corp.internal", "10.99.0.7")

    with subtest("and networkd restarting"):
        machine.succeed("systemctl restart systemd-networkd")
        holds_address("10.43.0.")
        rules_stand()
        answers("probe.corp.internal", "10.99.0.7")
  '';
}
