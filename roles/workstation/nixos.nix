# A NixOS machine Joel writes code on, with the workstation role in every
# account.
{ nixosModules, homeModules }:
{
  imports = [ nixosModules.base ];
  home-manager.sharedModules = [ homeModules.workstation ];
}
