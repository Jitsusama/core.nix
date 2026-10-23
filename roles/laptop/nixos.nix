# A NixOS machine Joel carries: Colemak on its own keyboard, power profiles
# that follow the charger, and wifid, over networkd and wpa_supplicant, for
# whichever network it meets, with network-revive giving his own services
# another go when it does, a lid that suspends it except at a desk, and a
# cable plugged back in that wakes it.
{ nixosModules }:
{
  imports = [
    nixosModules.base
    nixosModules.keyd
    nixosModules.lid
    nixosModules.network-revive
    nixosModules.power-profiles-daemon
    nixosModules.usb-wake
    nixosModules.wifid
  ];
}
