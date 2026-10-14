# A NixOS machine Joel carries: Colemak on its own keyboard, power profiles
# that follow the charger, and wifid, over networkd and wpa_supplicant, for
# whichever network it meets, with network-revive giving his own services
# another go when it does.
{ nixosModules }:
{
  imports = [
    nixosModules.base
    nixosModules.keyd
    nixosModules.network-revive
    nixosModules.power-profiles-daemon
    nixosModules.wifid
  ];
}
