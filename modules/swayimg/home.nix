# swayimg, for a quick look at a picture: a screenshot, a download, a
# photo, a drawing. It's light and quick to open, and it's only a look:
# editing is GIMP's and Inkscape's, a key away.
#
# Its settings are a Lua script, init.lua, which stays Lua. The theme reaches
# it through nix.lua beside it, written here: the theme's colours as the
# numbers swayimg takes, its proportional face and sizes, and the programs
# the keys run. Behind a picture is the deepest colour, so the picture is
# what's seen, with a quiet checkerboard under anything transparent. The name
# and its place in the folder show over it in the theme's face for two
# seconds as it opens, so nothing sits on the picture for long, and the
# gallery marks the one selected with the accent's hairline, lifted a
# little so it's found on a busy picture.
#
# It opens floating in the middle of the screen, a look rather than a place
# to work, and Escape or q puts it away. Its keys are swayimg's own, Enter
# going between the picture and the gallery of its folder, with these:
# e edits in GIMP, or Inkscape for a drawing, and closes the viewer; y
# copies the picture as shown; Delete moves it to the trash rather than
# deleting it; q closes it, as in mpv.
#
# The launcher finds the screenshots folder's gallery by typing
# (Screenshots.qml).
{ niri, quickshell }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme)
    roles
    font
    type
    space
    shape
    ;

  # A colour as swayimg's number, 0xAARRGGBB, written as a number rather
  # than a string, opaque unless the theme or the caller gives it an alpha.
  argb =
    alpha: colour:
    let
      hex = lib.removePrefix "#" colour;
      own = builtins.substring 6 2 hex;
    in
    lib.generators.mkLuaInline "0x${if own == "" then alpha else own}${builtins.substring 0 6 hex}";

  opaque = argb "ff";

  # Points as Linux counts them, 96 to the inch, to the pixels swayimg takes.
  pixels = points: builtins.floor (builtins.fromJSON points * 4 / 3 + 0.5);

  lua = lib.generators.toLua { };

  given = {
    colour = {
      deepest = opaque roles.deepest;
      sunken = opaque roles.sunken;
      raised = opaque roles.raised;
      text = opaque roles.text;
      strong = opaque roles.strong;
      muted = opaque roles.muted;
      accent = opaque roles.accent;
      # Under the name, so it reads on any picture.
      shadow = argb "d0" roles.shadow;
    };
    sans = font.sans.family;
    size = pixels type.sans.large;
    space = {
      inherit (space) s l;
    };
    inherit (shape) border;
    # A gallery cell, about ten across a window the width of the screen.
    thumbnail = 160;
    wl_copy = lib.getExe' pkgs.wl-clipboard "wl-copy";
    gio = lib.getExe' pkgs.glib "gio";
    gimp = {
      name = "GIMP";
      command = "gimp";
    };
    inkscape = {
      name = "Inkscape";
      command = "inkscape";
    };
  };
in
{
  imports = [
    ../theme/home.nix
    niri
    quickshell
  ];

  programs.swayimg = {
    enable = true;
    initLua = ./init.lua;
  };

  xdg.configFile."swayimg/nix.lua".text = ''
    -- Written by modules/swayimg/home.nix from jitsusama.theme.
    return ${lua given}
  '';

  # After niri's own rules, which match no window of swayimg's. It opens at
  # four fifths of the screen, centred, with the desktop showing around it:
  # swayimg's own size is a fixed 1280 by 720, too wide for a laptop's
  # screen at twice its scale.
  xdg.configFile."niri/config.kdl".text = lib.mkAfter ''
    window-rule {
        match app-id="^swayimg$"
        open-floating true
        default-column-width { proportion 0.8; }
        default-window-height { proportion 0.8; }
    }
  '';

  # After GIMP's, since it's about the same pictures.
  jitsusama.quickshell.launcher.providers.swayimg = {
    order = 47;
    source = ./Screenshots.qml;
    folder = "${config.home.homeDirectory}/Pictures/Screenshots";
  };
}
