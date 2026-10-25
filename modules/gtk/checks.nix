# The checks of GTK's settings, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  graphical,
  holds,
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
}
