# GTK programs in the theme: libadwaita's, such as GNOME's portal dialogs, and
# GTK 3's, such as GIMP's and Inkscape's windows.
# Both read their colours from a stylesheet of Joel's that outranks their
# theme, written here from jitsusama.theme. The desktop says whether it
# prefers dark or light, as the theme is, which every program that asks the
# portal hears, websites included. Like Omarchy's themes, they keep Adwaita's
# shapes and use the theme's Yaru icons; their words are in the theme's sans
# face and monospaced text in its monospaced one, as every other program's.
# libadwaita's colours:
# https://gnome.pages.gitlab.gnome.org/libadwaita/doc/1.9/css-variables.html
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme)
    colours
    font
    shape
    appearance
    icons
    floating
    ;
  rgba = import ../theme/rgba.nix { inherit lib; };

  # Each surface's background and text, and the colours of buttons and
  # messages that mean something, with the theme's text for drawing on them.
  palette = {
    window_bg_color = colours.background;
    window_fg_color = colours.text;
    view_bg_color = colours.sunken;
    view_fg_color = colours.text;
    # A window's frame sits a step deeper than its content, focused or not.
    # Chrome draws its tab strip in this and the current tab in the
    # window's colour, which is how the current tab stands out.
    headerbar_bg_color = colours.deepest;
    headerbar_backdrop_color = colours.deepest;
    headerbar_fg_color = colours.text;
    sidebar_bg_color = colours.sunken;
    # Cards, such as a calculator's display or a settings group, raised
    # from the window rather than libadwaita's white wash over it.
    card_bg_color = colours.raised;
    card_fg_color = colours.text;
    sidebar_fg_color = colours.text;
    dialog_bg_color = colours.background;
    dialog_fg_color = colours.text;
    # Menus and popovers float over the window in the colour its frame is
    # drawn in, as the shell's floating panels start from it, rather than
    # in the window's own colour with only a hairline between them.
    popover_bg_color = colours.deepest;
    popover_fg_color = colours.text;
    accent_bg_color = colours.accent;
    accent_fg_color = colours.on_accent;
    destructive_bg_color = colours.alert;
    destructive_fg_color = colours.on_accent;
    success_bg_color = colours.success;
    success_fg_color = colours.on_accent;
    warning_bg_color = colours.warning;
    warning_fg_color = colours.on_accent;
    error_bg_color = colours.alert;
    error_fg_color = colours.on_accent;
  };

  # libadwaita and adw-gtk3 both draw from these names, and a user stylesheet
  # outranks the theme's own definitions.
  stylesheet =
    lib.concatLines (lib.mapAttrsToList (name: color: "@define-color ${name} ${color};") palette)
    + tooltips
    + lib.optionalString (appearance == "dark") knobs;

  # A tooltip drawn as the shell's floating panels are, the frame's colour
  # seen a little through with a hairline round it and the theme's text,
  # rather than both themes' black wash, white words and their shadow. GTK 3
  # colours the words of a tooltip's every part, Chromium's included.
  tooltips = ''
    tooltip.background, tooltip.background.chromium {
      background-color: ${rgba colours.deepest (toString floating.opacity)};
      border: ${toString shape.border}px solid ${colours.rule};
      border-radius: ${toString shape.radius}px;
      color: ${colours.text};
      text-shadow: none;
    }
    tooltip * { color: ${colours.text}; }
  '';

  # A slider's and a switch's knob, which both themes mix from white rather
  # than the palette: the theme's text, its strongest under the pointer and
  # muted when it can't be moved. A scale with marks draws its knob as a
  # picture of a pin, which a colour would only box in, so it keeps its own.
  # On a light theme white is the paper's, so those keep theirs.
  knobs =
    let
      scale = "scale:not(.marks-before):not(.marks-after)";
    in
    ''
      ${scale} slider, switch slider { background-color: ${colours.text}; }
      ${scale} slider:hover, switch:hover slider { background-color: ${colours.strong}; }
      ${scale} slider:disabled, switch:disabled slider { background-color: ${colours.muted}; }
    '';
in
{
  imports = [ ../theme/home.nix ];

  gtk = {
    enable = true;
    colorScheme = appearance;
    # adw-gtk3 draws GTK 3 as libadwaita draws GTK 4, which needs no theme.
    theme = {
      name = if appearance == "dark" then "adw-gtk3-dark" else "adw-gtk3";
      package = pkgs.adw-gtk3;
    };
    gtk4.theme = null;
    iconTheme = {
      name = icons;
      package = pkgs.yaru-theme;
    };
    gtk3.extraCss = stylesheet;
    gtk4.extraCss = ''
      ${stylesheet}
      /* Floating windows' corners; tiled ones are square already. GTK 4
         programs without libadwaita draw with adw-gtk3's, which sets the
         accent to GNOME's blue itself rather than from its name. */
      :root {
        --window-radius: ${toString shape.radius}px;
        --accent-bg-color: @accent_bg_color;
        --accent-fg-color: @accent_fg_color;
      }
    '';
  };

  # GTK 3 and 4 read their words' face from their settings, and libadwaita
  # and Chrome from the desktop's, through the portal, which this writes
  # too; GTK's own default is whatever fontconfig calls sans.
  gtk.font = {
    name = font.sans.family;
    size = builtins.fromJSON font.sans.size;
  };

  dconf.settings."org/gnome/desktop/interface" = {
    monospace-font-name = "${font.mono.family} ${font.mono.size}";
    # Letters drawn in their own shapes rather than nudged onto whole
    # pixels, as macOS draws them: on a screen at twice the scale the
    # nudge only thins them. GTK and Chrome read it through the portal;
    # Joel judged Chrome's GitHub better for it on 2026-10-24.
    font-hinting = "none";
  };
}
