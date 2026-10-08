# A NixOS machine Joel writes code on, with the workstation role in every
# account.
{ nixosModules, homeModules }:
{
  imports = [
    nixosModules.base
    nixosModules.memory
  ];
  home-manager.sharedModules = [ homeModules.workstation ];
}
