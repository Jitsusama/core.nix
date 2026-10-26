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
# ? lists every key, on the statistics' page of them; the launcher lists the
# ones worth knowing over its window, in their own words. It reports what
# it plays over MPRIS, so the media keys and the shell's player reach it.
# umpv, the package's second entry, which queues files into one window,
# stays out of the launcher, where it would sit beside mpv as a second mpv.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) roles font;

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
  # Playback speed, those that change how loud as Loudness, and the rest
  # loose.
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
        chord = "9";
        does = "Quieter";
      }
      {
        chord = "0";
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
      {
        chord = "Return";
        does = "The next in the list";
      }
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
    ../theme/home.nix
    ../unlisted/home.nix
  ];

  # The launcher lists these over mpv's window, by the same words.
  jitsusama.keys.mpv = map (key: {
    inherit (key) chord does;
    group = key.group or "";
  }) keys;

  jitsusama.unlisted = [ "umpv" ];

  programs.mpv = {
    enable = true;
    scripts = [ pkgs.mpvScripts.mpris ];
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
      osd-color = roles.strong;
      osd-border-color = roles.shadow;
      osd-border-size = 2;
      osd-back-color = roles.deepest;
      osd-selected-color = roles.on_accent;
      osd-selected-outline-color = roles.accent;
      sub-font = font.sans.family;
      sub-color = roles.strong;
      sub-border-color = roles.shadow;

      screenshot-directory = "${config.home.homeDirectory}/Pictures/Screenshots";
      screenshot-format = "png";
      screenshot-template = "%F %wH-%wM-%wS";
    };
    scriptOpts.osc = {
      background_color = roles.deepest;
      timecode_color = roles.text;
      title_color = roles.strong;
      time_pos_color = roles.text;
      buttons_color = roles.text;
      small_buttonsL_color = roles.text;
      small_buttonsR_color = roles.text;
      top_buttons_color = roles.text;
      held_element_color = roles.muted;
      time_pos_outline_color = roles.shadow;
    };
    # The console, where a command is typed and a list is chosen from, and
    # the menu a right click opens: the choice in the accent, as the
    # launcher's is.
    scriptOpts.console = {
      monospace_font = font.mono.family;
      focused_color = roles.on_accent;
      focused_back_color = roles.accent;
      match_color = roles.accent;
      menu_outline_color = roles.edge;
    };
    scriptOpts.context_menu = {
      focused_color = roles.on_accent;
      focused_back_color = roles.accent;
      disabled_color = roles.muted;
      menu_outline_color = roles.edge;
    };
    # What's playing and how, and the page of keys ? opens, in the sans
    # face with figures in the monospaced one, its graphs in the accent.
    scriptOpts.stats = {
      font = font.sans.family;
      font_mono = font.mono.family;
      font_color = bgr roles.text;
      border_color = bgr roles.shadow;
      plot_color = bgr roles.accent;
      plot_bg_color = bgr roles.deepest;
      plot_bg_border_color = bgr roles.edge;
    };
  };
}
