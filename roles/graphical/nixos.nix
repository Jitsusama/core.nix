# A NixOS machine with a screen in front of Joel: fonts, sound, Bluetooth for
# headphones and mice, a keyring the TPM unlocks, 1Password, and niri, started
# at boot by greetd. Every account gets niri's configuration, kitty, Quickshell
# for the launcher, notifications and lock screen, which swayidle locks, and
# GTK programs and the monospace default in the theme. It also gets the
# programs Joel draws, watches, listens and pairs with: GIMP, Inkscape, which
# opens SVG drawings, mpv, which opens video, Spotify and Tuple. The
# compositor and sound get the CPU and memory before anything Joel starts.
{ nixosModules, homeModules }:
{
  imports = [
    nixosModules.base
    nixosModules.bluetooth
    nixosModules.fonts
    nixosModules.gnome-keyring
    nixosModules.greetd
    nixosModules.inkscape
    nixosModules.mpv
    nixosModules.niri
    nixosModules.onepassword
    nixosModules.pipewire
    nixosModules.quickshell
    nixosModules.slices
  ];
  home-manager.sharedModules = [
    homeModules.fontconfig
    homeModules.gimp
    homeModules.gtk
    homeModules.inkscape
    homeModules.niri
    homeModules.kitty
    homeModules.mpv
    homeModules.quickshell
    homeModules.spotify
    homeModules.swayidle
    homeModules.tuple
  ];
}
