# A NixOS machine Joel carries: Colemak on its own keyboard, and power
# profiles that follow the charger.
{ nixosModules }:
{
  imports = [
    nixosModules.base
    nixosModules.keyd
    nixosModules.power-profiles-daemon
  ];
}
