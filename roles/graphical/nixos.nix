# A NixOS machine with a screen in front of Joel: fonts, sound, and niri, with
# niri's configuration in every account.
{ nixosModules, homeModules }:
{
  imports = [
    nixosModules.base
    nixosModules.fonts
    nixosModules.niri
    nixosModules.pipewire
  ];
  home-manager.sharedModules = [ homeModules.niri ];
}
