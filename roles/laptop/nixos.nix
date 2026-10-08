# A NixOS machine Joel carries: Colemak on its own keyboard, for now.
{ nixosModules }:
{
  imports = [
    nixosModules.base
    nixosModules.keyd
  ];
}
