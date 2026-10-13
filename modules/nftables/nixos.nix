# The firewall on nftables, the kernel's packet filter, rather than the
# iptables commands that translate into it. libvirt's networks and Podman's
# follow it there, so every program's rules sit in one ruleset, each in a
# table of its own, and `nft list ruleset` reads them all.
#
# What still speaks iptables keeps working through nf_tables' compatibility
# layer: k3s's network policies and Docker's rules among them.
{
  networking.nftables.enable = true;

  # A reload deletes only the tables NixOS declares. nixpkgs flushes the
  # whole ruleset instead as soon as a machine adds a raw one, which would
  # take WARP's, libvirt's and Podman's tables with it, and their traffic.
  networking.nftables.flushRuleset = false;
}
