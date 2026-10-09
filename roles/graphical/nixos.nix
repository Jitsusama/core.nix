# A NixOS machine with a screen in front of Joel: fonts, sound, Bluetooth for
# headphones and mice, and niri, started at boot by greetd. Every account gets
# niri's configuration, kitty, and Quickshell for the launcher, notifications
# and lock screen, which swayidle locks.
{ nixosModules, homeModules }:
{
  imports = [
    nixosModules.base
    nixosModules.bluetooth
    nixosModules.fonts
    nixosModules.greetd
    nixosModules.niri
    nixosModules.pipewire
    nixosModules.quickshell
  ];
  home-manager.sharedModules = [
    homeModules.niri
    homeModules.kitty
    homeModules.quickshell
    homeModules.swayidle
  ];
}
