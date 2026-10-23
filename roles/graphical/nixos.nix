# A NixOS machine with a screen in front of Joel: fonts, sound, Bluetooth for
# headphones and mice, a plugged-in monitor's own controls, which programs
# stream from a USB camera, a desk light lit while one does, a keyring the TPM unlocks, 1Password, and niri, started
# at boot by greetd, with the console, the consoles signed in on, and
# Plymouth, which asks for the disk's PIN, in the theme of the account it
# signs in. Every account gets
# niri's configuration, kitty, Quickshell for the launcher, notifications
# and lock screen, which swayidle locks, a backlight that follows the
# light, and GTK programs and the monospace default in the theme. It also
# gets the programs Joel views, draws, reads, watches, listens and pairs
# with: swayimg, which opens pictures and drawings, zathura, which opens
# documents, GIMP, Inkscape, mpv, which opens video, the camera, Spotify
# and Tuple. When a program asks what to open something with, the launcher
# asks rather than GNOME's dialog. The compositor and sound get the CPU and memory before
# anything Joel starts.
{ nixosModules, homeModules }:
{
  imports = [
    nixosModules.backlight
    nixosModules.base
    nixosModules.bluetooth
    nixosModules.camera
    nixosModules.console
    nixosModules.ddcutil
    nixosModules.fonts
    nixosModules.gnome-keyring
    nixosModules.greetd
    nixosModules.kmscon
    nixosModules.litra
    nixosModules.mpv
    nixosModules.niri
    nixosModules.onepassword
    nixosModules.open-with
    nixosModules.pipewire
    nixosModules.plymouth
    nixosModules.quickshell
    nixosModules.shure-mv7
    nixosModules.slices
    nixosModules.swayimg
    nixosModules.usb-camera-holders
    nixosModules.zathura
  ];
  home-manager.sharedModules = [
    homeModules.backlight
    homeModules.camera
    homeModules.fontconfig
    homeModules.gimp
    homeModules.gtk
    homeModules.inkscape
    homeModules.niri
    homeModules.kitty
    homeModules.open-with
    homeModules.mpv
    homeModules.qt
    homeModules.quickshell
    homeModules.spotify
    homeModules.swayidle
    homeModules.swayimg
    homeModules.tuple
    homeModules.zathura
  ];
}
