# The checks of systemd-networkd, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  self,
  ...
}:
{
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
