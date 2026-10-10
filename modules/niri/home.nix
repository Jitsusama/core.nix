# Joel's niri: config.kdl says how it behaves, and theme.kdl, written here from
# jitsusama.theme, how it looks. config.kdl is text a machine can add to, with
# lib.mkAfter, such as a panel's scale; niri lets a later setting override an
# earlier one, so the machine's lines win.
#
# The cursor is Adwaita's, as Omarchy's is through its system default. niri
# draws it and tells every program it starts which theme and size to use, and
# its default names a theme called "default" that nothing installs, so without
# this only niri's built-in arrow shows and every other shape (resize, grab,
# text, not-allowed) falls back to it.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) colors shape;
  cursor = {
    name = "Adwaita";
    package = pkgs.adwaita-icon-theme;
    size = 24;
  };
in
{
  imports = [ ../theme/home.nix ];

  # Also links it as ~/.icons/default and names it to GTK, for programs that
  # look there rather than at niri's variables. Home Manager offers this on
  # Linux only, and niri runs nowhere else.
  home.pointerCursor = lib.mkIf pkgs.stdenv.hostPlatform.isLinux (cursor // { gtk.enable = true; });

  xdg.configFile."niri/config.kdl".text = builtins.readFile ./config.kdl;

  xdg.configFile."niri/theme.kdl".text = ''
    layout {
        gaps ${toString shape.gap}
        background-color "${colors.background}"
        border {
            width ${toString shape.border}
            active-color "${colors.accent}"
            // Omarchy's inactive border, which its palette doesn't name.
            inactive-color "#595959aa"
        }
        tab-indicator {
            active-color "${colors.accent}"
            inactive-color "${colors.lighter_background}"
        }
        insert-hint {
            color "${colors.accent}80"
        }
    }
    window-rule {
        geometry-corner-radius ${toString shape.radius}
        clip-to-geometry true
    }
    overview {
        backdrop-color "${colors.darker_background}"
    }
    cursor {
        xcursor-theme "${cursor.name}"
        xcursor-size ${toString cursor.size}
    }
  '';
}
