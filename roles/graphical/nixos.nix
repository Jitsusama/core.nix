# A NixOS machine with a screen in front of Joel: fonts, sound, Bluetooth for
# headphones and mice, and niri, with niri's configuration and kitty in every
# account.
{ nixosModules, homeModules }:
{
  imports = [
    nixosModules.base
    nixosModules.bluetooth
    nixosModules.fonts
    nixosModules.niri
    nixosModules.pipewire
  ];
  home-manager.sharedModules = [
    homeModules.niri
    homeModules.kitty
  ];
}
