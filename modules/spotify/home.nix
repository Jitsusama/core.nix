# Spotify in the theme, and in the terminal. Spotify's own client comes
# through Spicetify, which rewrites the colours the client draws in to
# ones it takes from a scheme, here jitsusama.theme's colours, and adds a
# stylesheet, here setting the theme's faces where Spotify sets its own. The
# client is built that way rather than patched in place, so nothing touches
# it after it's installed, and Spotify keeps its own prefs file, which holds
# the account it signs in with.
#
# The launcher lists Spotify's keys over its window, as Music. Spotify
# takes no settings for its keys, so a Spicetify extension of ours,
# speaks-vims-keys.js, gives its page the key language's
# (modules/keys/language.toml): Vim's keys scroll it, Ctrl+o and Ctrl+i go
# back and forward, and / filters its list, where Spotify's own Ctrl+f now
# goes down a page. Escape stays Spotify's, closing and letting go, since it
# never steps out.
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
#
# In the launcher, Programs › Spotify holds the playlists, albums and
# shows Spotify played lately as its Recent, and Liked Songs, Playlists,
# Albums, Artists and Podcasts inside it, each played from there
# (SpotifyProgram.qml), read through spotify-recent with spotify-player's
# sign-in only as Spotify is stepped into.
{ spicetify-nix, quickshell }:
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

  # What each of the language's acts speaks-vims-keys.js does in Spotify
  # does, in the launcher's words, scrolling or not.
  scrollActs = {
    down = "Scroll down";
    up = "Scroll up";
    scroll-down = "Scroll down a line";
    scroll-up = "Scroll up a line";
    half-down = "Down half a page";
    half-up = "Up half a page";
    screen-down = "Down a page";
    screen-up = "Up a page";
    first = "The top of the page";
    last = "The bottom of the page";
  };
  otherActs = {
    jump-back = "Back a page";
    jump-forward = "Forward a page";
    search-forward = "Filter the list";
  };

  # Each act's keys speaks-vims-keys.js reads: letters and Ctrl's, not a
  # named key, as Page Down, which Spotify takes by itself.
  keysOf =
    act:
    lib.filter (
      key: builtins.match "<C-.>|[^<]+" key != null
    ) config.jitsusama.keyLanguage.acts.${act}.keys;

  # Every key it takes, with its act and what that does.
  bindsOf =
    acts:
    lib.concatMap (
      act:
      map (key: {
        inherit act key;
        does = acts.${act};
      }) (keysOf act)
    ) (lib.attrNames acts);
  scrollBinds = bindsOf scrollActs;
  otherBinds = bindsOf otherActs;

  # What speaks-vims-keys.js reads: each key's act, and how long a g waits
  # for its second.
  language = {
    inherit (config.jitsusama.keyLanguage.timing) wait;
    keys = lib.listToAttrs (
      map (bind: lib.nameValuePair bind.key bind.act) (scrollBinds ++ otherBinds)
    );
  };

  # A bind of the extension's, in the launcher's spelling.
  listed = bind: {
    chord = chordOf bind.key;
    inherit (bind) act does;
  };

  # A key in the launcher's spelling: <C-d> is Ctrl+d, G is Shift+g and gg
  # is g g.
  chordOf =
    key:
    lib.concatMapStringsSep " " (
      token:
      let
        ctrl = builtins.match "<C-(.)>" token;
      in
      if ctrl != null then
        "Ctrl+${lib.head ctrl}"
      else if lib.toLower token != token then
        "Shift+${lib.toLower token}"
      else
        token
    ) (map lib.head (lib.filter lib.isList (builtins.split "(<C-.>|.)" key)));

  # Spotify's own keys, which it takes no settings for, from its list for
  # Windows and Linux (support.spotify.com's keyboard shortcuts), and the
  # ones speaks-vims-keys.js adds.
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
    ++ grouped "scrolling" (map listed scrollBinds)
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
    ++ map listed otherBinds
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
    quickshell
    spicetify-nix.homeManagerModules.spicetify
  ];

  # Spotify builds its client for x86_64 Linux and Macs alone, so a machine
  # it isn't built for, such as an Apple laptop on Linux, plays through
  # spotify-player and the web player instead.
  programs.spicetify = {
    enable = lib.meta.availableOn pkgs.stdenv.hostPlatform pkgs.spotify;
    customColorScheme = lib.mapAttrs (_: lib.removePrefix "#") scheme;
    # Spicetify's plain theme, which only recolours, with the faces added.
    theme = spices.themes.default // {
      additionalCss = faces;
    };
    enabledExtensions = [
      {
        src = pkgs.writeTextDir "speaks-vims-keys.js" (
          lib.replaceStrings [ ''"@language@"'' ] [ (builtins.toJSON (builtins.toJSON language)) ] (
            builtins.readFile ./speaks-vims-keys.js
          )
        );
        name = "speaks-vims-keys.js";
      }
    ];
  };

  # Spotify's keys, for the launcher to list over its window, whose app id
  # is spotify.
  jitsusama.keys.spotify = keys;

  # Spotify's Recent and what Joel keeps there in Programs › Spotify,
  # made beside Slack's program and the other programs' acts. Each is played
  # through spotify-recent while a Spotify runs, else opened in the
  # client; signed out, Sign In runs spotify-player's sign-in in kitty,
  # since it prints where to sign in and waits for Spotify's answer. Where
  # Spotify's client isn't built, as on an Apple laptop on Linux, none is
  # named, and one is opened in the web player instead.
  jitsusama.quickshell.launcher.providers.spotify-program = {
    order = 47;
    source = ./SpotifyProgram.qml;
    command = [ (lib.getExe (pkgs.callPackage ./spotify-recent/package.nix { })) ];
    spotify = lib.optional config.programs.spicetify.enable (
      lib.getExe config.programs.spicetify.spicedSpotify
    );
    signIn = [
      "kitty"
      (lib.getExe config.programs.spotify-player.package)
      "authenticate"
    ];
  };

  # What the provider works out, beside it, where its import finds it.
  xdg.configFile."quickshell/launcher/Spotify.js".source = ./Spotify.js;

  programs.spotify-player = {
    enable = true;
    # spotify-player signs in through ncspot's Spotify app, which since
    # 2026-09-28 takes only http://127.0.0.1:8989/ncspot_login back, so its
    # sign-in stopped at "redirect_uri: Not matching configuration". The
    # patch (ncspot-sign-in.patch) asks for that address for ncspot's app
    # alone, keeping /login for the streaming client's sign-in, which is
    # Spotify's own app. Upstream's aome510/spotify-player#1108 changes only
    # the first half; take this out once a release signs in. The patch also
    # has `spotify_player authenticate` sign the streaming client in with
    # the token it gets, which is what saves the credentials it plays with;
    # it dropped the token, so a sign-in by that command alone never played.
    package = pkgs.spotify-player.overrideAttrs (old: {
      patches = (old.patches or [ ]) ++ [ ./ncspot-sign-in.patch ];
    });
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
