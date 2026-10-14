# network-revive for every account home-manager keeps, on a machine whose
# network networkd runs, since networkd is what it follows.
{
  imports = [ ../systemd-networkd/nixos.nix ];

  home-manager.sharedModules = [ ./home.nix ];
}
