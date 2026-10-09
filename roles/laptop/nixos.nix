# A NixOS machine Joel carries: Colemak on its own keyboard, power profiles
# that follow the charger, and NetworkManager for whichever network it meets.
{ nixosModules }:
{
  imports = [
    nixosModules.base
    nixosModules.keyd
    nixosModules.networkmanager
    nixosModules.power-profiles-daemon
  ];
}
