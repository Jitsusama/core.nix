# systemd-networkd gives every network a laptop meets its addresses, routes
# and DNS, and resolved answers for them. Each network is a .network file:
# the two here say how a stranger's Wi-Fi or cable is treated, and a saved
# network is a file of its own that sorts before them, written by Nix or by
# wifid. Links no file matches, such as a VPN's tunnel, a bridge or a
# container's veth, are left to whoever made them.
{
  networking.useNetworkd = true;
  # NixOS would add catch-alls of its own for DHCP; these are the only ones.
  networking.useDHCP = false;
  services.resolved.enable = true;

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
    # no two networks can tell they've met the same machine. Nothing goes in
    # resolved's global settings, which would reach every resolver on the
    # machine, so LLMNR and mDNS are turned off here, link by link.
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

    # The same for a cable or a phone's USB tether. Kind=!* leaves out veths,
    # taps and bridges, which are Ethernet too.
    "90-ethernet" = {
      matchConfig = {
        Type = "ether";
        Kind = "!*";
      };
      networkConfig = {
        DHCP = "yes";
        IPv6PrivacyExtensions = "yes";
        LLMNR = "no";
        MulticastDNS = "no";
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
  };

  # The kernel's Wi-Fi stack reads each country's channel and power limits
  # from here; without it, it falls back to the most restrictive set.
  hardware.wirelessRegulatoryDatabase = true;
}
