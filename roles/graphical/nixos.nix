# A NixOS machine with a screen in front of Joel: fonts, sound, Bluetooth for
# headphones and mice, a keyring the TPM unlocks, 1Password, and niri, started
# at boot by greetd. Every account gets niri's configuration, kitty, Quickshell
# for the launcher, notifications and lock screen, which swayidle locks, and
# GTK programs and the monospace default in the theme. It also gets the
# programs Joel views, draws, reads, watches, listens and pairs with:
# swayimg, which opens pictures and drawings, zathura, which opens
# documents, GIMP, Inkscape, mpv, which opens video, the camera, Spotify and
# Tuple. The compositor and sound get the CPU and memory before anything Joel
# starts.
{ nixosModules, homeModules }:
{
  imports = [
    nixosModules.base
    nixosModules.bluetooth
    nixosModules.camera
    nixosModules.fonts
    nixosModules.gnome-keyring
    nixosModules.greetd
    nixosModules.mpv
    nixosModules.niri
    nixosModules.onepassword
    nixosModules.pipewire
    nixosModules.quickshell
    nixosModules.slices
    nixosModules.swayimg
    nixosModules.zathura
  ];
  home-manager.sharedModules = [
    homeModules.camera
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
    homeModules.swayimg
    homeModules.tuple
    homeModules.zathura
  ];
}
