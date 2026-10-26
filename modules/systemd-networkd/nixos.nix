# systemd-networkd gives every network a laptop meets its addresses, routes
# and DNS, and resolved answers for them. Each network is a .network file:
# the two here say how a stranger's Wi-Fi or cable is treated, and a saved
# network is a file of its own that sorts before them, written by Nix or by
# wifid. Links no file matches, such as a VPN's tunnel, a bridge or a
# container's veth, are left to whoever made them.
{ config, lib, ... }:
let
  # A stranger's cable or a phone's USB tether, treated as 90-wifi treats a
  # stranger's Wi-Fi. Kind=!* leaves out veths, taps and bridges, which are
  # Ethernet too.
  ethernet = {
    matchConfig = {
      Type = "ether";
      Kind = "!*";
    };
    networkConfig = {
      DHCP = "yes";
      IPv6PrivacyExtensions = "yes";
      LLMNR = "no";
      MulticastDNS = "no";
      # A dock's hub drops the cable's carrier for a moment as it resets;
      # without this, every connection and tunnel lost its addresses with
      # it, and resolved logged 265 lines of trouble in one such hour.
      IgnoreCarrierLoss = "3s";
    };
    dhcpV4Config = {
      ClientIdentifier = "mac";
      SendHostname = false;
      UseHostname = false;
      RouteMetric = 100;
    };
    dhcpV6Config = {
      SendHostname = false;
      UseHostname = false;
    };
    ipv6AcceptRAConfig.RouteMetric = 100;
  };

  # A cable trusted as wifid trusts a saved Wi-Fi network: the same as a
  # stranger's, but matched by the Ethernet card's address and with mDNS on,
  # sorting before 90-ethernet.
  trusted = address: {
    name = "50-ethernet-${lib.replaceStrings [ ":" ] [ "" ] (lib.toLower address)}";
    value = lib.recursiveUpdate ethernet {
      matchConfig.MACAddress = address;
      networkConfig.MulticastDNS = "yes";
    };
  };
in
{
  options.jitsusama.network.trustedCables = lib.mkOption {
    type = lib.types.listOf (lib.types.strMatching "([0-9A-Fa-f]{2}:){5}[0-9A-Fa-f]{2}");
    default = [ ];
    example = [ "44:67:52:00:00:01" ];
    description = ''
      The hardware addresses of Ethernet cards whose network is trusted,
      set per machine: a dock's card at home, say. On a trusted cable the
      machine finds others by their .local names and is found by its own,
      as on a Wi-Fi network wifid trusts; on any other cable it's neither.
    '';
  };

  config = {
    networking.useNetworkd = true;
    # NixOS would add catch-alls of its own for DHCP; these are the only ones.
    networking.useDHCP = false;
    services.resolved.enable = true;

    # LLMNR is the one thing turned off for every link rather than link by
    # link, since links nothing here manages, a VPN's tunnel or a bridge,
    # keep resolved's default of asking it. A bare name nobody answers, a
    # typo or an ssh to a host the network doesn't know, then waited on
    # LLMNR's timeouts: 8.8 s before glibc said no, where a name that
    # doesn't exist failed through DNS in under a second. No network here
    # answers LLMNR.
    services.resolved.settings.Resolve.LLMNR = "no";

    # mDNS asks and answers on UDP 5353, and the firewall dropped both the
    # answers to this machine's questions and other machines' questions, so a
    # trusted network found nothing and nothing found it. The port is open on
    # every link, but only a trusted network's file turns mDNS on, and resolved
    # ignores an mDNS packet that arrives on a link with it off; on a
    # stranger's network, nothing is there to answer.
    networking.firewall.allowedUDPPorts = [ 5353 ];

    # networkd removes every routing rule it didn't make whenever it sets up a
    # link. It isn't the only program routing here: WARP, Tailscale and
    # wg-quick each send their traffic through a rule of their own, and would
    # lose it on every join. It already leaves other links' routes alone.
    systemd.network.config.networkConfig.ManageForeignRoutingPolicyRules = false;

    # Waiting for every link would stall boot on a laptop with no network, and
    # waiting for none would start what needs the network too early. From
    # cold, a laptop's Wi-Fi was online fifteen seconds after this began, so
    # ten seconds gave up on a network that came. Thirty is what
    # NetworkManager's nm-online waited. wifid turns this off for a wait of
    # its own, which knows when a card has nothing to join.
    systemd.network.wait-online = {
      anyInterface = true;
      timeout = 30;
    };

    systemd.network.networks = {
      # A stranger's Wi-Fi learns as little as it can, and changes nothing
      # beyond the link: no hostname sent or taken, and an
      # identity that follows the address wpa_supplicant gives this network, so
      # no two networks can tell they've met the same machine. mDNS is turned
      # off here, link by link, rather than in resolved's global settings, so
      # a saved network can turn it back on; LLMNR is off everywhere.
      "90-wifi" = {
        matchConfig = {
          Type = "wlan";
          WLANInterfaceType = "station";
        };
        networkConfig = {
          DHCP = "yes";
          IPv6PrivacyExtensions = "yes";
          LLMNR = "no";
          MulticastDNS = "no";
          # Moving between access points of one network drops the carrier for a
          # moment; keeping the addresses spares every connection, and the
          # tunnels watching the routes.
          IgnoreCarrierLoss = "3s";
        };
        dhcpV4Config = {
          ClientIdentifier = "mac";
          SendHostname = false;
          UseHostname = false;
          # A cable beats Wi-Fi: Ethernet's routes weigh 100 to these 600, as
          # NetworkManager weighed them.
          RouteMetric = 600;
        };
        dhcpV6Config = {
          DUIDType = "link-layer";
          SendHostname = false;
          UseHostname = false;
        };
        ipv6AcceptRAConfig.RouteMetric = 600;
      };

      "90-ethernet" = ethernet;
    }
    // lib.listToAttrs (map trusted config.jitsusama.network.trustedCables);

    # Docked, the cable and Wi-Fi hold an address each on the same network.
    # The kernel answers ARP for either address on either link, and names
    # whichever it likes as the source of its own ARP, so the router can tie
    # Wi-Fi's address to the cable's hardware and lose it at an undock until
    # ARP refreshes; a call's peer on the same network then sees one machine
    # at two paths that are one. Each link answers for its own addresses only
    # (arp_ignore 1) and names its own address when it asks (arp_announce 2).
    boot.kernel.sysctl = {
      "net.ipv4.conf.all.arp_ignore" = 1;
      "net.ipv4.conf.default.arp_ignore" = 1;
      "net.ipv4.conf.all.arp_announce" = 2;
      "net.ipv4.conf.default.arp_announce" = 2;
    };

    # The kernel's Wi-Fi stack reads each country's channel and power limits
    # from here; without it, it falls back to the most restrictive set.
    hardware.wirelessRegulatoryDatabase = true;
  };
}
