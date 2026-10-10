# GTK programs in the theme: libadwaita's, such as GNOME's portal dialogs, and
# GTK 3's, such as the file chooser the GTK portal opens for Chrome and Slack.
# Both read their colours from a stylesheet of Joel's that outranks their
# theme, written here from jitsusama.theme. The desktop says it prefers dark,
# which every program that asks the portal hears, websites included. Like
# Omarchy's Osaka Jade, they keep Adwaita's shapes and its sans interface font
# and use the Yaru-sage icons; monospaced text is in the theme's font.
# libadwaita's colours:
# https://gnome.pages.gitlab.gnome.org/libadwaita/doc/1.9/css-variables.html
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) roles font shape;

  # Each surface's background and text, and the colours of buttons and
  # messages that mean something, with the theme's text for drawing on them.
  palette = {
    window_bg_color = roles.background;
    window_fg_color = roles.text;
    view_bg_color = roles.sunken;
    view_fg_color = roles.text;
    headerbar_bg_color = roles.background;
    headerbar_fg_color = roles.text;
    sidebar_bg_color = roles.sunken;
    sidebar_fg_color = roles.text;
    dialog_bg_color = roles.background;
    dialog_fg_color = roles.text;
    popover_bg_color = roles.background;
    popover_fg_color = roles.text;
    accent_bg_color = roles.accent;
    accent_fg_color = roles.on_accent;
    destructive_bg_color = roles.alert;
    destructive_fg_color = roles.on_accent;
    success_bg_color = roles.success;
    success_fg_color = roles.on_accent;
    warning_bg_color = roles.warning;
    warning_fg_color = roles.on_accent;
    error_bg_color = roles.alert;
    error_fg_color = roles.on_accent;
  };

  # libadwaita and adw-gtk3 both draw from these names, and a user stylesheet
  # outranks the theme's own definitions.
  stylesheet = lib.concatLines (
    lib.mapAttrsToList (name: color: "@define-color ${name} ${color};") palette
  );
in
{
  imports = [ ../theme/home.nix ];

  gtk = {
    enable = true;
    colorScheme = "dark";
    # adw-gtk3 draws GTK 3 as libadwaita draws GTK 4, which needs no theme.
    theme = {
      name = "adw-gtk3-dark";
      package = pkgs.adw-gtk3;
    };
    gtk4.theme = null;
    iconTheme = {
      name = "Yaru-sage";
      package = pkgs.yaru-theme;
    };
    gtk3.extraCss = stylesheet;
    gtk4.extraCss = ''
      ${stylesheet}
      /* Floating windows' corners; tiled ones are square already. */
      :root {
        --window-radius: ${toString shape.radius}px;
      }
    '';
  };

  dconf.settings."org/gnome/desktop/interface".monospace-font-name =
    "${font.mono.family} ${font.mono.size}";
}
