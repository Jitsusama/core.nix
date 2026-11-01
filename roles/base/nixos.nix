# Every NixOS machine: home-manager inside the system, nixpkgs' settings, the
# firewall on nftables, a name service that finds every address family, an
# sshd that lets a gone caller go, a clock that keeps time on any network,
# mounts that don't wait on systemd and zsh as the login shell, with the base
# role in every account.
{ nixosModules, homeModules }:
{
  imports = [
    nixosModules.home-manager
    nixosModules.name-service
    nixosModules.nftables
    nixosModules.nixpkgs
    nixosModules.openssh
    nixosModules.systemd
    nixosModules.timesyncd
    nixosModules.zsh
  ];
  home-manager.sharedModules = [ homeModules.base ];
}
