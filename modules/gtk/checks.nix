# The checks of GTK's settings, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  graphical,
  holds,
  lib,
  onLinux,
  ...
}:
onLinux {
  # The desktop tells GTK and Chrome, through the portal, to draw letters
  # unhinted, as macOS does; a desktop that says nothing gets slight hinting.
  the-desktop-draws-letters-unhinted =
    holds "the-desktop-draws-letters-unhinted"
      "the graphical account's dconf no longer sets org.gnome.desktop.interface font-hinting to none"
      (((account graphical).dconf.settings."org/gnome/desktop/interface".font-hinting or null) == "none");

  # GTK's words are in the theme's sans face, as the desktop tells libadwaita
  # and Chrome and as GTK's own settings say, and fontconfig's sans-serif is
  # that face too, for any program that names none.
  gtk-speaks-in-the-theme =
    let
      home = account graphical;
      sans = home.jitsusama.theme.font.sans;
    in
    holds "gtk-speaks-in-the-theme"
      "GTK, the desktop's dconf or fontconfig's sans-serif no longer names the theme's sans face"
      (
        lib.hasPrefix "${sans.family} " (home.dconf.settings."org/gnome/desktop/interface".font-name or "")
        && (home.gtk.font.name or null) == sans.family
        && home.fonts.fontconfig.defaultFonts.sansSerif == [ sans.family ]
      );
}
