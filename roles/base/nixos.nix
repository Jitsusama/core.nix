# Every NixOS machine: home-manager inside the system, nixpkgs' settings, the
# firewall on nftables and zsh as the login shell, with the base role in every
# account.
{ nixosModules, homeModules }:
{
  imports = [
    nixosModules.home-manager
    nixosModules.nftables
    nixosModules.nixpkgs
    nixosModules.zsh
  ];
  home-manager.sharedModules = [ homeModules.base ];
}
