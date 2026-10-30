# mpv plays video, set for the picture first and then for smooth motion:
#
# - It draws through libplacebo (gpu-next), on Vulkan where there is one,
#   and decodes on the GPU wherever that's known to be safe, so a 4K file
#   plays without the CPU.
# - The high-quality profile scales with a sharpened EWA Lanczos, and
#   banding in flat colour is smoothed out, as web video is full of it.
# - Frames are timed to the display rather than to the sound, with the
#   sound resampled by a fraction of a percent to keep in step, so no frame
#   is shown twice or dropped to catch up. A frame rate that doesn't divide
#   the display's is blended between frames rather than juddered. The sound
#   stays in sync over Bluetooth too: PipeWire tells mpv how long the
#   headphones hold it, and mpv shows each frame that much later.
# - A file stays on its last frame when it ends, so a short recording can
#   be looked at rather than vanishing. Sound alone opens no window.
#   Quitting remembers where a file was, and opening it again goes back.
#
# What mpv draws over the picture is in the theme: its messages and
# subtitles in the proportional face, the controller's bar in the deepest
# colour with the text's colour on it, and its console, right-click menu
# and statistics on the deepest colour, a choice in the accent. The keys are
# mpv's own: Space pauses, the arrows seek, f fills the screen, q quits and
# ? lists every key, on the statistics' page of them, and 1 to 9 play that
# one in the list, as numbers choose that one everywhere, rather than change
# the picture's contrast, brightness, gamma and saturation as mpv has them;
# the launcher lists the
# ones worth knowing over its window, in their own words. It reports what
# it plays over MPRIS, so the media keys and the shell's player reach it,
# and says each file it plays to the desktop's list of files opened lately
# (recent-files.lua), so its Recent in the launcher lists them, led by
# the file each mpv plays now (playing.lua), which Return goes to.
# umpv, the package's second entry, which queues files into one window,
# stays out of the launcher, where it would sit beside mpv as a second mpv.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) colours font;

  # Where each mpv keeps the file it plays now (playing.lua).
  playing = "${config.xdg.stateHome}/mpv/playing";

  # The theme's #RRGGBB as stats writes a colour, blue first.
  bgr =
    colour:
    let
      hex = lib.removePrefix "#" colour;
    in
    lib.concatMapStrings (at: builtins.substring at 2 hex) [
      4
      2
      0
    ];

  # The keys worth knowing, as niri spells them, each as mpv binds it by
  # default: the launcher lists them over mpv's window, those that move
  # through what's playing as Seeking, those that change how fast as
  # Playback speed, those that change how loud as Loudness, those that
  # choose from what it was given as In the list, and the rest loose; a key
  # with a run is bound to it here, the rest are mpv's own.
  grouped = group: map (key: key // { inherit group; });
  keys =
    grouped "seeking" [
      {
        chord = "Left";
        does = "Back 5 seconds";
      }
      {
        chord = "Right";
        does = "On 5 seconds";
      }
      {
        chord = "Down";
        does = "Back a minute";
      }
      {
        chord = "Up";
        does = "On a minute";
      }
      {
        chord = "Page_Down";
        does = "The chapter before";
      }
      {
        chord = "Page_Up";
        does = "The next chapter";
      }
    ]
    ++ grouped "speed" [
      {
        chord = "bracketleft";
        does = "Slower";
      }
      {
        chord = "bracketright";
        does = "Faster";
      }
      {
        chord = "BackSpace";
        does = "At its own speed";
      }
    ]
    ++ grouped "loudness" [
      {
        chord = "/";
        does = "Quieter";
      }
      {
        chord = "*";
        does = "Louder";
      }
      {
        chord = "m";
        does = "Silent, or heard again";
      }
    ]
    ++ [
      {
        chord = "Space";
        does = "Pause, or play on";
      }
      {
        chord = "f";
        does = "Fill the screen, or not";
      }
    ]
    # The list mpv was given, by number and in turn.
    ++ grouped "list" (
      map (number: {
        chord = toString number;
        does = "Number ${toString number} in the list";
        run = "set playlist-pos ${toString (number - 1)}";
      }) (lib.range 1 9)
      ++ [
        {
          chord = "Return";
          does = "The next in the list";
        }
        {
          chord = "Shift+comma";
          does = "The one before in the list";
        }
      ]
    )
    ++ [
      {
        chord = "s";
        does = "Save the frame as a picture";
      }
      {
        chord = "Shift+slash";
        does = "The player's keys";
      }
      {
        chord = "q";
        does = "Close the player";
      }
    ];
in
{
  imports = [
    ../keys/home.nix
    ../quickshell/recents.nix
    ../theme/home.nix
    ../unlisted/home.nix
  ];

  # The launcher lists these over mpv's window, by the same words.
  jitsusama.keys.mpv = map (key: {
    inherit (key) chord does;
    group = key.group or "";
  }) keys;

  jitsusama.unlisted = [ "umpv" ];

  xdg.configFile."mpv/scripts/recent-files.lua".source = ./recent-files.lua;
  xdg.configFile."mpv/scripts/playing.lua".source = ./playing.lua;

  # The file each mpv plays now, for its Recent in the launcher to lead
  # with, kept in a folder made at sign-in (playing.lua), each mpv's line
  # printed as the launcher reads it, as mpv is stepped into. The launcher
  # runs it without a shell, so the folder is named here, in the state
  # home rather than the runtime folder, whose path only a shell knows;
  # what a killed mpv left is cleared by the next to start, and the
  # launcher lists a file only while its mpv has a window. Its other paths
  # stay the desktop's list of files opened lately.
  systemd.user.tmpfiles.rules = [ "d ${playing} 0700 - - -" ];
  jitsusama.quickshell.launcher.recents = [
    {
      app = "mpv";
      kind = "file";
      opened = import ../quickshell/kept-in.nix { inherit lib pkgs; } playing;
    }
  ];

  programs.mpv = {
    enable = true;
    scripts = [ pkgs.mpvScripts.mpris ];
    # The keys whose meaning is ours rather than mpv's, from the list above.
    # mpv's 0, louder beside 9's quieter, does nothing now that 9 plays the
    # ninth in the list; / and * stay mpv's quieter and louder.
    bindings =
      lib.listToAttrs (map (key: lib.nameValuePair key.chord key.run) (lib.filter (key: key ? run) keys))
      // {
        "0" = "ignore";
      };
    config = {
      profile = "high-quality";
      vo = "gpu-next";
      hwdec = "auto-safe";
      deband = true;

      video-sync = "display-resample";
      interpolation = true;
      tscale = "oversample";
      ao = "pipewire";

      keep-open = true;
      save-position-on-quit = true;
      title = "\${media-title}";

      osd-font = font.sans.family;
      osd-font-size = 24;
      osd-color = colours.strong;
      osd-border-color = colours.shadow;
      osd-border-size = 2;
      osd-back-color = colours.deepest;
      osd-selected-color = colours.on_accent;
      osd-selected-outline-color = colours.accent;
      sub-font = font.sans.family;
      sub-color = colours.strong;
      sub-border-color = colours.shadow;

      screenshot-directory = "${config.home.homeDirectory}/Pictures/Screenshots";
      screenshot-format = "png";
      screenshot-template = "%F %wH-%wM-%wS";
    };
    scriptOpts.osc = {
      background_color = colours.deepest;
      timecode_color = colours.text;
      title_color = colours.strong;
      time_pos_color = colours.text;
      buttons_color = colours.text;
      small_buttonsL_color = colours.text;
      small_buttonsR_color = colours.text;
      top_buttons_color = colours.text;
      held_element_color = colours.muted;
      time_pos_outline_color = colours.shadow;
    };
    # The console, where a command is typed and a list is chosen from, and
    # the menu a right click opens: the choice in the accent, as the
    # launcher's is.
    scriptOpts.console = {
      monospace_font = font.mono.family;
      focused_color = colours.on_accent;
      focused_back_color = colours.accent;
      match_color = colours.accent;
      menu_outline_color = colours.edge;
    };
    scriptOpts.context_menu = {
      focused_color = colours.on_accent;
      focused_back_color = colours.accent;
      disabled_color = colours.muted;
      menu_outline_color = colours.edge;
    };
    # What's playing and how, and the page of keys ? opens, in the sans
    # face with figures in the monospaced one, its graphs in the accent.
    scriptOpts.stats = {
      font = font.sans.family;
      font_mono = font.mono.family;
      font_color = bgr colours.text;
      border_color = bgr colours.shadow;
      plot_color = bgr colours.accent;
      plot_bg_color = bgr colours.deepest;
      plot_bg_border_color = bgr colours.edge;
    };
  };
}
