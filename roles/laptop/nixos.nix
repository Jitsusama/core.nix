# A NixOS machine Joel carries: Colemak on its own keyboard, power profiles
# that follow the charger, and wifid, over networkd and wpa_supplicant, for
# whichever network it meets, with network-revive giving his own services
# another go when it does, a lid that suspends it except at a desk, and a
# cable plugged back in that wakes it. It hibernates with a fast compressor
# for its image. A paired Bluetooth keyboard types at its disk prompt. Each
# account can read the home network's eero, at home or away.
{ nixosModules, homeModules }:
{
  imports = [
    nixosModules.base
    nixosModules.bluetooth-disk-prompt
    nixosModules.hibernate
    nixosModules.keyd
    nixosModules.lid
    nixosModules.network-revive
    nixosModules.power-profiles-daemon
    nixosModules.usb-wake
    nixosModules.wifid
  ];
  home-manager.sharedModules = [
    homeModules.eero
  ];
}
