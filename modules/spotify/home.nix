# Spotify in the theme, and in the terminal. Spotify's own client comes
# through Spicetify, which rewrites the colours the client draws in to
# ones it takes from a scheme, here jitsusama.theme's colours, and adds a
# stylesheet, here setting the theme's faces where Spotify sets its own. The
# client is built that way rather than patched in place, so nothing touches
# it after it's installed, and Spotify keeps its own prefs file, which holds
# the account it signs in with.
#
# The launcher lists Spotify's own keys over its window, as Music. Spotify
# takes no settings for its keys, and it has every key of the desktop's but
# one: Escape only closes and lets go there. So a Spicetify extension of
# ours, backs-out.js, makes Escape go back a page once nothing is left to
# close or let go of, as it backs out a step everywhere else.
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
  inherit (config.jitsusama.theme) colours font;
  spices = spicetify-nix.legacyPackages.${pkgs.stdenv.hostPlatform.system};

  # What each colour Spicetify names is for, in Spotify's own client.
  scheme = {
    inherit (colours) text;
    subtext = colours.muted;
    main = colours.background;
    main-elevated = colours.raised;
    highlight = colours.raised;
    highlight-elevated = colours.selection;
    sidebar = colours.sunken;
    player = colours.sunken;
    card = colours.raised;
    inherit (colours) shadow;
    # Drawn faint behind a hovered row, so it's the text's colour.
    selected-row = colours.text;
    button = colours.accent;
    button-active = colours.strong;
    button-disabled = colours.rule;
    tab-active = colours.selection;
    notification = colours.raised;
    notification-error = colours.alert;
    misc = colours.rule;
  };

  grouped = group: map (key: key // { inherit group; });

  # Spotify's own keys, which it takes no settings for, from its list for
  # Windows and Linux (support.spotify.com's keyboard shortcuts), and the
  # one backs-out.js adds.
  keys =
    grouped "going" [
      {
        chord = "Alt+Shift+h";
        does = "Home";
      }
      {
        chord = "Alt+Shift+0";
        does = "Your library";
      }
      {
        chord = "Alt+Shift+1";
        does = "Your playlists";
      }
      {
        chord = "Alt+Shift+2";
        does = "Your podcasts";
      }
      {
        chord = "Alt+Shift+3";
        does = "Your artists";
      }
      {
        chord = "Alt+Shift+4";
        does = "Your albums";
      }
      {
        chord = "Alt+Shift+5";
        does = "Your audiobooks";
      }
      {
        chord = "Alt+Shift+s";
        does = "Your liked songs";
      }
      {
        chord = "Alt+Shift+m";
        does = "Made for you";
      }
      {
        chord = "Alt+Shift+n";
        does = "New releases";
      }
      {
        chord = "Alt+Shift+c";
        does = "The charts";
      }
      {
        chord = "Alt+Shift+j";
        does = "What's playing now";
      }
      {
        chord = "Alt+Shift+q";
        does = "The queue";
      }
    ]
    ++ grouped "playing" [
      {
        chord = "Space";
        does = "Play or pause the music";
      }
      {
        chord = "Ctrl+s";
        does = "Shuffle on or off";
      }
      {
        chord = "Ctrl+r";
        does = "Repeat on or off";
      }
      {
        chord = "m";
        does = "Mute or unmute the music";
      }
      {
        chord = "Alt+Shift+b";
        does = "Like the song, or not";
      }
    ]
    ++ [
      {
        chord = "Ctrl+k";
        does = "Find anything in Spotify";
      }
      {
        chord = "Ctrl+l";
        does = "The search page";
      }
      {
        chord = "Ctrl+f";
        does = "Filter the list";
      }
      {
        chord = "Alt+j";
        does = "The row's menu";
      }
      {
        chord = "Alt+Shift+r";
        does = "Show or hide what's playing beside";
      }
      {
        chord = "Alt+Shift+l";
        does = "Show or hide the library beside";
      }
      {
        chord = "Escape";
        does = "Back in Spotify";
      }
      {
        chord = "Ctrl+,";
        does = "Spotify's settings";
      }
      {
        chord = "?";
        does = "Spotify's own list of keys";
      }
    ];

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
    ../keys/home.nix
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
    enabledExtensions = [
      {
        src = pkgs.writeTextDir "backs-out.js" (builtins.readFile ./backs-out.js);
        name = "backs-out.js";
      }
    ];
  };

  # Spotify's keys, for the launcher to list over its window, whose app id
  # is spotify.
  jitsusama.keys.spotify = keys;

  programs.spotify-player = {
    enable = true;
    settings = {
      # A notice for every new track is one Joel didn't ask for.
      enable_notify = false;
      # Rounded, as the theme's corners are.
      border_type = "Rounded";
    };
  };

  # Spotify pauses while a call app records from a microphone and plays
  # again once the call ends, as Joel chose: a call app takes out only its
  # own sound, so music from the speakers would reach the other end.
  # pause-for-calls/src/main.rs says how it knows.
  systemd.user.services.pause-for-calls = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
    Unit = {
      Description = "Pauses Spotify while a call app records from a microphone";
      After = [ "pipewire.service" ];
      BindsTo = [ "pipewire.service" ];
    };
    Service = {
      ExecStart = lib.getExe (pkgs.callPackage ./pause-for-calls/package.nix { });
      Environment = [
        "PATH=${
          lib.makeBinPath [
            pkgs.pipewire
            pkgs.systemd
          ]
        }"
      ];
      Restart = "on-failure";
      RestartSec = 2;
    };
    Install.WantedBy = [ "pipewire.service" ];
  };
}
