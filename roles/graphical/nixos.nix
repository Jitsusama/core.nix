# A NixOS machine with a screen in front of Joel: fonts, for now.
{ nixosModules }:
{
  imports = [
    nixosModules.base
    nixosModules.fonts
  ];
}
