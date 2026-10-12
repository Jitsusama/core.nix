# Spotify in the theme, and in the terminal. Spotify's own client comes
# through Spicetify, which rewrites the colours the client draws in to
# ones it takes from a scheme, here jitsusama.theme's roles, and adds a
# stylesheet, here setting the theme's faces where Spotify sets its own. The
# client is built that way rather than patched in place, so nothing touches
# it after it's installed, and Spotify keeps its own prefs file, which holds
# the account it signs in with.
#
# The shell shows each new track itself (Music.qml). Spotify's own notice
# for one is off unless it's turned on in Spotify's settings, and Spotify
# keeps that choice in prefs, a file it rewrites from memory as it quits, so
# it's left to Spotify. Spotify doesn't take that pref from its command line.
#
# spotify-player plays and browses Spotify from the terminal, as a Spotify
# Connect device of its own. It draws in the terminal's colours, which are
# the theme's, so it needs no theme of its own, and it reports what's
# playing to the desktop's media keys.
{ spicetify-nix }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) roles font;
  spices = spicetify-nix.legacyPackages.${pkgs.stdenv.hostPlatform.system};

  # What each colour Spicetify names is for, in Spotify's own client.
  scheme = {
    inherit (roles) text;
    subtext = roles.muted;
    main = roles.background;
    main-elevated = roles.raised;
    highlight = roles.raised;
    highlight-elevated = roles.selection;
    sidebar = roles.sunken;
    player = roles.sunken;
    card = roles.raised;
    inherit (roles) shadow;
    # Drawn faint behind a hovered row, so it's the text's colour.
    selected-row = roles.text;
    button = roles.accent;
    button-active = roles.strong;
    button-disabled = roles.rule;
    tab-active = roles.selection;
    notification = roles.raised;
    notification-error = roles.alert;
    misc = roles.rule;
  };

  # Spotify names its faces once each, on :root, and this comes after.
  faces = ''
    :root {
      --encore-body-font-stack: "${font.sans.family}", sans-serif;
      --encore-title-font-stack: "${font.sans.family}", sans-serif;
      --encore-variable-font-stack: "${font.sans.family}", sans-serif;
      --encore-bodyMono-font-stack: "${font.mono.family}", monospace;
    }
  '';
in
{
  imports = [
    ../theme/home.nix
    spicetify-nix.homeManagerModules.spicetify
  ];

  programs.spicetify = {
    enable = true;
    customColorScheme = lib.mapAttrs (_: lib.removePrefix "#") scheme;
    # Spicetify's plain theme, which only recolours, with the faces added.
    theme = spices.themes.default // {
      additionalCss = faces;
    };
  };

  programs.spotify-player = {
    enable = true;
    settings = {
      # A notice for every new track is one Joel didn't ask for.
      enable_notify = false;
      # Rounded, as zellij draws its panes.
      border_type = "Rounded";
    };
  };
}
