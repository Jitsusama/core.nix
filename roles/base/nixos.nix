# Every NixOS machine: home-manager inside the system and nixpkgs' settings,
# with the base role in every account.
{ nixosModules, homeModules }:
{
  imports = [
    nixosModules.home-manager
    nixosModules.nixpkgs
  ];
  home-manager.sharedModules = [ homeModules.base ];
}
