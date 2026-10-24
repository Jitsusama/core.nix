# The checks of nftables, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  holds,
  workstation,
  ...
}:
{
  # Every program's rules sit in nftables, each in a table of its own, and a
  # reload deletes only the firewall's: flushing the ruleset would take the
  # others' tables, and their traffic, with it. A raw ruleset is what turns
  # nixpkgs' flush on, so the machine here adds one.
  one-ruleset-for-every-program =
    let
      machine =
        (workstation.extendModules {
          modules = [ { networking.nftables.ruleset = "table inet probe { }"; } ];
        }).config;
      containers = machine.virtualisation.containers.containersConf.settings;
    in
    holds "one-ruleset-for-every-program"
      "A program's rules leave nftables, or a reload flushes the others' tables."
      (
        machine.networking.firewall.enable
        && machine.networking.nftables.enable
        && !machine.networking.nftables.flushRuleset
        && machine.virtualisation.libvirtd.firewallBackend == "nftables"
        && containers.network.firewall_driver == "nftables"
      );
}
