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
# What mpv draws over the picture is in the theme: its messages, times
# and statistics in the monospaced face, as names and figures are
# everywhere, subtitles in the serif upright, as words to read are (its
# cut of its own, ../fonts/serif-text.nix, since mpv can't set a variable
# font's axes), the controller's bar in the deepest colour with the
# text's colour on it, and its console, right-click menu
# and statistics on the deepest colour, a choice in the accent. Its
# messages are the Figure level's size at every size of window, the inset
# from its frame, on kitty's text column (osd-by-scale.lua, and
# osd-lands-on-pixels.patch, without which no margin lands there). The keys are
# Vim's, as Joel's Vim has them (vi-keys.lua), each with one meaning, and
# mpv's own where Vim has nothing: Space pauses at once, f fills the screen
# and Escape leaves it, and ? and F1 open the launcher's Shortcuts at the
# player's keys. h and l seek back and on 5 seconds, j on a minute and k
# back, as j goes on through a file, and each arrow does what its letter
# does, though no list names one; 0 goes to the start and $ to the end, where the last frame stays,
# gg and G play the first and the last in the list, ]b and [b the next and
# the one before, a number before a key does it that many times and before
# G plays that one, and yy copies the frame as a picture (copy-frame.lua).
# q closes nothing, as in all his programs, and says what does: ZZ. Space
# leads nothing here, so there's no Space w, as there is elsewhere. j and l were
# mpv's subtitles and A-B loop, which go to J, where mpv cycles subtitles
# already, and L; the digits no longer change the picture's contrast,
# brightness, gamma and saturation, nor the loudness, as mpv has them.
# The launcher lists the
# ones worth knowing over its window, in their own words. It reports what
# it plays over MPRIS, so the media keys and the shell's player reach it,
# and says each file it plays to the desktop's list of files opened lately
# (recent-files.lua), so its Recent in the launcher lists them, led by
# the file each mpv plays now (playing.lua), which Return goes to. Among
# them, by when mpv left each, are the files it stopped part way through,
# each saying where, which Return resumes, and those it played to their
# end, which Return plays from the start (MpvProgram.qml). While the
# clipboard holds an address it plays, a video site's page or a media
# file, Programs › mpv offers Play the Copied Link, and only then.
# umpv, the package's second entry, which queues files into one window,
# stays out of the launcher, where it would sit beside mpv as a second mpv.
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
    type
    space
    shape
    ;

  # Its messages at the Figure level, the mono's large step, in logical
  # pixels. libass sizes a face by its line, its ascent and descent, which
  # are 1.245 of MonaspiceXe's em; the capitals' top sits 0.265 of an em
  # below the line's, and an H's ink 0.033 in from its origin. The margins
  # are the inset less those, so the ink stands the inset from the frame,
  # on kitty's text column; another face needs measuring again.
  em = builtins.floor (builtins.fromJSON type.mono.large * 4 / 3 + 0.5);
  message = {
    size = em * 1.245;
    margin_x = space.rung.inset - 3.3e-2 * em;
    margin_y = space.rung.inset - 0.265 * em;
    outline = 2 * shape.border;
  };
  serifText = import ../fonts/serif-text.nix { inherit pkgs font; };

  # Where each mpv keeps the file it plays now (playing.lua).
  playing = "${config.xdg.stateHome}/mpv/playing";
  # Where mpv keeps where it stopped in each file, named here so mpv and
  # the launcher's lister of them (stopped.sh) read one folder.
  watchLater = "${config.xdg.stateHome}/mpv/watch_later";

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
  # with a run is bound to it here, on its key as mpv spells one (`key`)
  # where that isn't its chord, Vim's are vi-keys.lua's, and the rest are
  # mpv's own. A key that twins another says the same words after it, so
  # the launcher lists both in one row, and does the same.
  grouped = group: map (key: key // { inherit group; });
  keys =
    grouped "seeking" [
      {
        chord = "h";
        does = "Back 5 seconds";
      }
      {
        chord = "l";
        does = "On 5 seconds";
      }
      {
        chord = "j";
        does = "On a minute";
      }
      {
        chord = "k";
        does = "Back a minute";
      }
      {
        chord = "Home";
        does = "The start";
      }
      {
        chord = "0";
        does = "The start";
      }
      # keep-open holds the last frame there.
      {
        chord = "$";
        does = "The end";
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
      # A moment after them with no b, as Vim's timeout.
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
        does = "Full screen, and out of it";
      }
      {
        chord = "Escape";
        does = "The video out of full screen";
      }
      # Where j and l were.
      {
        chord = "Shift+j";
        key = "J";
        does = "The next subtitles";
        run = "cycle sub";
      }
      {
        chord = "Shift+l";
        key = "L";
        does = "Repeat between two points, set in turn";
        run = "ab-loop";
      }
    ]
    # The list mpv was given, in turn and by number: ]b and [b as Vim's go
    # between buffers.
    ++ grouped "list" [
      {
        chord = "Return";
        does = "The next in the list";
      }
      {
        chord = "bracketright b";
        does = "The next in the list";
      }
      {
        chord = "Shift+comma";
        does = "The one before in the list";
      }
      {
        chord = "bracketleft b";
        does = "The one before in the list";
      }
      {
        chord = "g g";
        does = "The first in the list";
      }
      {
        chord = "Shift+g";
        does = "The last in the list";
      }
      # Any number before G, three drawn for them all.
      {
        chord = "3 Shift+g";
        does = "That one in the list, by its number";
      }
    ]
    # The frame in view, as a picture; y alone waits for what it's to copy,
    # as in Vim, and does nothing.
    ++ grouped "frame" [
      {
        chord = "y y";
        does = "Copy the frame as a picture";
      }
      {
        chord = "s";
        does = "Save the frame as a picture";
      }
      {
        chord = "Shift+s";
        does = "Save the frame without its subtitles";
      }
    ]
    ++ [
      {
        chord = "Shift+slash";
        does = "The player's keys";
      }
      # As in Joel's Neovim, where Space ? does it too; here Space leads
      # nothing.
      {
        chord = "F1";
        does = "The player's keys";
      }
      # As everywhere, q closes nothing; Space leads nothing here, so ZZ
      # alone closes.
      {
        chord = "Z Z";
        does = "Close the player";
      }
      {
        chord = "Shift+colon";
        does = "A command for the video";
      }
    ];
in
{
  imports = [
    ../keys/home.nix
    ../quickshell/recents.nix
    ../quickshell/command-lines.nix
    ../theme/home.nix
    ../unlisted/home.nix
  ];

  # The launcher lists these over mpv's window, by the same words.
  jitsusama.keys.mpv = map (key: {
    inherit (key) chord does;
    group = key.group or "";
  }) keys;

  jitsusama.unlisted = [ "umpv" ];

  # The serif upright cut for the subtitles, where fontconfig finds it.
  home.packages = [ serifText ];

  xdg.configFile."mpv/scripts/recent-files.lua".source = ./recent-files.lua;
  xdg.configFile."mpv/scripts/playing.lua".source = ./playing.lua;
  xdg.configFile."mpv/scripts/copy-frame.lua".source = ./copy-frame.lua;
  xdg.configFile."mpv/scripts/vi-keys.lua".source = ./vi-keys.lua;
  xdg.configFile."mpv/scripts/osd-by-scale.lua".source = ./osd-by-scale.lua;
  # What mpv's provider works out, beside it, where its import finds it.
  xdg.configFile."quickshell/launcher/Mpv.js".source = ./Mpv.js;

  # The files mpv left, in its Recent, and Play the Copied Link, among its
  # acts; after Slack's, beside the desktop's actions, as GIMP's and
  # Tuple's are.
  jitsusama.quickshell.launcher.providers.mpv = {
    order = 48;
    source = ./MpvProgram.qml;
    # The programs daemon's key for the files it left, listed below.
    stopped = "listed:mpv:stopped";
  };

  # The file each mpv plays now, for its Recent in the launcher to lead
  # with, kept in a folder made at sign-in (playing.lua), each mpv's line
  # printed as the launcher reads it, as mpv is stepped into. The launcher
  # runs it without a shell, so the folder is named here, in the state
  # home rather than the runtime folder, whose path only a shell knows;
  # what a killed mpv left is cleared by the next to start, and the
  # launcher lists a file only while its mpv has a window. Its other paths
  # stay the desktop's list of files opened lately.
  systemd.user.tmpfiles.rules = [ "d ${playing} 0700 - - -" ];
  # : opens the launcher's command line over the video, with these
  # commands, each by the key language's name (../keys/language.toml), and
  # a time, :12:30, to go to it; mpv-command (command.sh) carries each.
  jitsusama.quickshell.launcher.commandLines.mpv = {
    commands = [
      {
        command = "set";
        title = "Set a property, as :set pause or :set volume=40";
      }
      {
        command = "yank";
        title = "Copy the frame as a picture";
        key = "yy";
      }
      {
        command = "quit";
        title = "Close the player";
        key = "ZZ";
      }
    ];
    goes = "To that time, as 12:30, or that many seconds in";
    run = [ (lib.getExe (pkgs.callPackage ./command.nix { })) ];
  };

  jitsusama.quickshell.launcher.recents = [
    {
      app = "mpv";
      kind = "file";
      opened = import ../quickshell/kept-in.nix { inherit lib pkgs; } playing;
    }
    # The files it left, which the programs daemon reads as its page asks.
    {
      app = "mpv";
      name = "stopped";
      asked = true;
      list = [
        (lib.getExe (pkgs.callPackage ./stopped.nix { }))
        watchLater
      ];
    }
  ];

  programs.mpv = {
    enable = true;
    # mpv lays its messages out on a 288-line grid, where no margin lands
    # the inset; the patch's own words say how it's mended.
    package = pkgs.mpv.override {
      mpv-unwrapped = pkgs.mpv-unwrapped.overrideAttrs (old: {
        patches = (old.patches or [ ]) ++ [ ./osd-lands-on-pixels.patch ];
      });
      scripts = [ pkgs.mpvScripts.mpris ];
    };
    # The keys whose meaning is ours rather than mpv's, from the list above;
    # / and * stay mpv's quieter and louder.
    bindings = lib.listToAttrs (
      map (key: lib.nameValuePair (key.key or key.chord) key.run) (lib.filter (key: key ? run) keys)
    );
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
      # Each file's where-it-stopped names the file, so the launcher can
      # list them (stopped.sh). mpv leaves it off for privacy, but each
      # path is in the desktop's list of files opened lately already.
      watch-later-dir = watchLater;
      write-filename-in-watch-later-config = true;
      title = "\${media-title}";

      osd-font = font.mono.family;
      # Its messages keep one size and room at every size of window, set
      # from logical pixels by osd-by-scale.lua.
      osd-scale-by-window = false;
      osd-color = colours.strong;
      osd-border-color = colours.shadow;
      osd-back-color = colours.deepest;
      osd-selected-color = colours.on_accent;
      osd-selected-outline-color = colours.accent;
      sub-font = font.serif.text.family;
      sub-color = colours.strong;
      sub-border-color = colours.shadow;

      screenshot-directory = "${config.home.homeDirectory}/Pictures/Screenshots";
      screenshot-format = "png";
      screenshot-template = "%F %wH-%wM-%wS";
    };
    scriptOpts.copy-frame.wl_copy = lib.getExe' pkgs.wl-clipboard "wl-copy";
    scriptOpts.osd-by-scale = message;
    # The controller, the console, its menu and the statistics keep growing
    # with the window, as they did before its messages stopped.
    scriptOpts.osc = {
      vidscale = "yes";
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
      scale_with_window = "yes";
      monospace_font = font.mono.family;
      focused_color = colours.on_accent;
      focused_back_color = colours.accent;
      match_color = colours.accent;
      menu_outline_color = colours.edge;
    };
    scriptOpts.context_menu = {
      scale_with_window = "yes";
      focused_color = colours.on_accent;
      focused_back_color = colours.accent;
      disabled_color = colours.muted;
      menu_outline_color = colours.edge;
    };
    # What's playing and how, the page i and I open, all in the
    # monospaced face, since it's labels beside figures, its graphs in
    # the accent.
    scriptOpts.stats = {
      vidscale = "yes";
      font = font.mono.family;
      font_mono = font.mono.family;
      font_color = bgr colours.text;
      border_color = bgr colours.shadow;
      plot_color = bgr colours.accent;
      plot_bg_color = bgr colours.deepest;
      plot_bg_border_color = bgr colours.edge;
    };
  };
}
