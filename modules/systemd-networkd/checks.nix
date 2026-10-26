# The checks of systemd-networkd, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  lib,
  self,
  ...
}:
{
  # A trusted cable is a stranger's cable in every way but two: it's matched
  # by its card's address, and mDNS is on, in a file that sorts before
  # 90-ethernet so networkd picks it first. A machine naming no cable gets
  # no such file. vm-machines-find-each-other-at-home shows that a link with
  # mDNS on finds and is found.
  a-trusted-cable-finds-machines =
    let
      address = "44:67:52:00:00:01";
      networksOf =
        cables:
        (bareNixos [
          self.nixosModules.systemd-networkd
          { jitsusama.network.trustedCables = cables; }
        ]).config.systemd.network.networks;
      plain = networksOf [ ];
      trusting = networksOf [ address ];
      name = "50-ethernet-446752000001";
      cable = trusting.${name} or null;
      stranger = trusting."90-ethernet";
      # What the module sets, as the rest of a network's options include
      # removed ones that refuse to be read.
      without = network: {
        matchConfig = removeAttrs network.matchConfig [ "MACAddress" ];
        networkConfig = removeAttrs network.networkConfig [ "MulticastDNS" ];
        inherit (network) dhcpV4Config dhcpV6Config ipv6AcceptRAConfig;
      };
      onlyEthernetFiles = networks: lib.filter (lib.hasPrefix "50-ethernet-") (lib.attrNames networks);
    in
    holds "a-trusted-cable-finds-machines"
      "A trusted cable's network isn't a stranger's with mDNS on, matched by its card and picked first, or one appears unasked."
      (
        onlyEthernetFiles plain == [ ]
        && cable != null
        && cable.matchConfig.MACAddress == address
        && cable.networkConfig.MulticastDNS == "yes"
        && stranger.networkConfig.MulticastDNS == "no"
        && without cable == without stranger
        && name < "90-ethernet"
      );

  # A development DNS server answers for its own domains through a file in
  # resolved.conf.d, so resolved's global settings stay as they ship: a
  # server, domain, DNSSEC or DNS over TLS set there would reach it as well.
  # What a network needs goes in its own .network file. The machine checked
  # has networkd without the laptop role, whose keyboard and power modules
  # leave resolved alone, so it holds whichever networking the role takes.
  resolved-is-set-per-network =
    let
      laptop = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        self.nixosModules.systemd-networkd
      ];
      resolve = laptop.config.services.resolved.settings.Resolve;
    in
    holds "resolved-is-set-per-network"
      "A machine with networkd sets resolved's global servers, domains, DNSSEC or DNS over TLS."
      (
        laptop.config.services.resolved.enable
        && resolve.DNS == [ ]
        && resolve.Domains == [ ]
        && resolve.DNSSEC == false
        && resolve.DNSOverTLS == false
      );
}
