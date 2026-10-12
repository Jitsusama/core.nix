# Quickshell draws everything on Joel's screen that isn't a window: the
# wallpaper, the launcher on Super+Space, notifications, the lock screen,
# polkit's password prompt, the volume, microphone, brightness and charge as
# they change, what's playing, and the time and status over the overview.
# Each is a QML file beside this one, linked as it is; Theme.qml, written
# here from jitsusama.theme, gives them all the same look. It runs with the
# graphical session, and restarts if it ever falls over. What the overview
# shows is
# jitsusama.quickshell.overview.pieces, rendered into Overview.qml at build,
# and what the launcher finds and how it learns are
# jitsusama.quickshell.launcher, rendered into Providers.qml.
{ wallpapers }:
{
  config,
  lib,
  pkgs,
  # The machine's own settings, which Home Manager gives when it's part of
  # one, for the weather's home: the time zone.
  osConfig ? { },
  ...
}:
let
  inherit (config.jitsusama.theme)
    roles
    font
    shape
    space
    type
    motion
    glyphs
    floating
    wallpaper
    ;

  # A picture the theme names, as a path in the wallpapers collection or an
  # absolute one, or nothing for none.
  picture =
    name:
    if name == null then
      ""
    else if lib.hasPrefix "/" name then
      name
    else
      "${wallpapers}/${name}";

  # A JavaScript object of the values, which QML reads as Theme.space.m.
  object = values: "(${builtins.toJSON values})";
  slowed = milliseconds: builtins.floor (milliseconds * builtins.fromJSON motion.slowdown + 0.5);
  face = family: sizes: object ({ inherit family; } // lib.mapAttrs (_: builtins.fromJSON) sizes);

  qml = [
    "shell.qml"
    "Prompt.qml"
    "Launcher.qml"
    "Choice.qml"
    "Margin.qml"
    "Notifications.qml"
    "Music.qml"
    "Music.js"
    "Lock.qml"
    "Polkit.qml"
    "Volume.qml"
    "Power.qml"
    "Brightness.qml"
    "Meter.qml"
    "Status.qml"
    "Facts.qml"
    "Wallpaper.qml"
    "Lifted.qml"
    "OpenBelow.qml"
    "Niri.qml"
    "Floating.qml"
    "Fuzzy.js"
    "Picks.qml"
    "Notices.qml"
    "Caffeinate.qml"
    "Quiet.qml"
    "Lights.qml"
    "pieces/Piece.qml"
    "launcher/Provider.qml"
    "launcher/Numbers.js"
    "launcher/Addresses.js"
    "launcher/Units.js"
    "launcher/Zones.js"
    "launcher/Colours.js"
    "launcher/Emoji.js"
  ];

  # A service holding one of logind's locks for as long as it runs, with a
  # file saying so.
  holding =
    {
      what,
      flag,
      ...
    }@unit:
    {
      Unit = removeAttrs unit [
        "what"
        "flag"
      ];
      Service = {
        ExecStart = lib.escapeShellArgs [
          (lib.getExe' pkgs.systemd "systemd-inhibit")
          "--what=${what}"
          "--who=Caffeinate"
          "--why=Kept awake from the launcher"
          "--mode=block"
          (lib.getExe' pkgs.coreutils "sleep")
          "infinity"
        ];
        ExecStartPost = "${lib.getExe' pkgs.coreutils "touch"} %t/${flag}";
        ExecStopPost = "${lib.getExe' pkgs.coreutils "rm"} -f %t/${flag}";
      };
    };

  # A part's corners, the whole's radius less its border and inset.
  inner = lib.max 0 (shape.radius - shape.border - shape.inset);

  # A mark from the icon font by its code point.
  nerd = import ../theme/nerd.nix { inherit lib; };

  # How a chord's keys are drawn: a key's own glyph where a proper one
  # exists, its name where none does, and a letter as itself (docs/design.md,
  # Shape and Kinds). Icon marks draw wider than a letter, so they're
  # marked to be given room.
  keyFaces = {
    Super = {
      text = nerd "f313";
      wide = true;
    };
    Shift.text = "⇧";
    Return.text = "↵";
    Up.text = "↑";
    Down.text = "↓";
    Left.text = "←";
    Right.text = "→";
    Escape.text = "Esc";
    Page_Up.text = "PgUp";
    Page_Down.text = "PgDn";
    Minus.text = "-";
    Equal.text = "=";
    Slash.text = "/";
    Period.text = ".";
    Semicolon.text = ";";
    Comma.text = ",";
    Grave.text = "`";
    BracketLeft.text = "[";
    BracketRight.text = "]";
    WheelScrollUp.text = "Wheel ↑";
    WheelScrollDown.text = "Wheel ↓";
    WheelScrollLeft.text = "Wheel ←";
    WheelScrollRight.text = "Wheel →";
    XF86AudioRaiseVolume = {
      text = nerd "f075d";
      wide = true;
    };
    XF86AudioLowerVolume = {
      text = nerd "f075e";
      wide = true;
    };
    XF86AudioMute = {
      text = nerd "f075f";
      wide = true;
    };
    XF86AudioMicMute = {
      text = nerd "f036d";
      wide = true;
    };
    XF86MonBrightnessUp = {
      text = nerd "f00e0";
      wide = true;
    };
    XF86MonBrightnessDown = {
      text = nerd "f00de";
      wide = true;
    };
    XF86AudioPrev = {
      text = nerd "f04ae";
      wide = true;
    };
    XF86AudioNext = {
      text = nerd "f04ad";
      wide = true;
    };
    XF86AudioPlay = {
      text = nerd "f040e";
      wide = true;
    };
  };
  face' = key: { wide = false; } // (keyFaces.${key} or { text = key; });

  # A chord in the key map's one spelling (super+shift+/), drawn key by key
  # as the desktop's are: a key with a face wears it, a letter is a capital,
  # and a word starts with one.
  chordFaces =
    keys:
    let
      faces = lib.mapAttrs' (name: lib.nameValuePair (lib.toLower name)) keyFaces // {
        enter = keyFaces.Return;
        pageup = keyFaces.Page_Up;
        pagedown = keyFaces.Page_Down;
      };
      key = if lib.hasSuffix "++" keys then "+" else lib.last (lib.splitString "+" keys);
      held = lib.init (lib.splitString "+" (lib.removeSuffix "+" keys));
      face =
        word:
        {
          wide = false;
        }
        // (faces.${word} or {
          text =
            if lib.stringLength word == 1 then
              lib.toUpper word
            else
              lib.toUpper (lib.substring 0 1 word) + lib.substring 1 (-1) word;
        }
        );
    in
    map face (held ++ [ key ]);

  # Every layer's shortcuts, top to bottom, for the launcher's collection of
  # them: what each does, its chord, and for the desktop's, the command that
  # does the same and the glyph its action wears.
  shortcutLayers =
    map
      (layer: {
        name = layer;
        glyph = layerGlyphs.${layer};
        binds = map (b: {
          inherit (b) does run;
          chord = chordFaces b.keys;
          glyph = if layer == "desktop" then actionGlyph b.run else "";
        }) config.jitsusama.keys.${layer};
      })
      [
        "desktop"
        "terminal"
        "multiplexer"
        "program"
      ];

  # The glyph an action wears in the launcher, by what it runs.
  actionGlyph =
    run:
    let
      program = if run == [ ] then "" else lib.head run;
      action = if program == "niri" then lib.elemAt run 3 else "";
      says = lib.concatStringsSep " " run;
    in
    if lib.hasInfix "kitty" says then
      nerd "f489"
    else if lib.hasPrefix "screenshot" action then
      nerd "f0e51"
    else if lib.hasInfix "AUDIO_SOURCE" says then
      nerd "f036d"
    else if program == "sh" && lib.hasInfix "wpctl" says then
      nerd "f057e"
    else if program == "brightnessctl" then
      nerd "f00e0"
    else if program == "qs" && lib.elem "music" run then
      nerd "f040e"
    else if lib.hasInfix "workspace" action || lib.hasInfix "overview" action then
      nerd "f0570"
    else if lib.hasInfix "monitor" action then
      nerd "f0379"
    # A collection's own glyph, as its row in the launcher wears.
    else if program == "qs" && lib.elem "collection" run then
      {
        clipboard = nerd "f0a38";
        emoji = nerd "f01f5";
        notices = nerd "f009c";
      }
      .${lib.last run} or (nerd "f097b")
    # The margin's drawer, which hangs from the top of the screen.
    else if program == "qs" && lib.elem "margin" run then
      nerd "f1513"
    else
      nerd "f2d0";

  # The desktop's binds the launcher can run, its own toggle left out, since
  # picking that from the launcher would only close it.
  # Each with the launcher's collection it opens, if it opens one, so the
  # launcher steps into it rather than closing to open again, and its home
  # screen shows the chord beside the collection.
  actions = map (b: {
    inherit (b) does run;
    glyph = actionGlyph b.run;
    chord = map face' (lib.splitString "+" b.chord);
    opens = if opensCollection b.run then lib.last b.run else "";
  }) (lib.filter (b: b.run != [ ] && b.run != launcherToggle) config.jitsusama.keys.desktop);
  opensCollection =
    run:
    lib.length run == 6
    &&
      lib.take 5 run == [
        "qs"
        "ipc"
        "call"
        "launcher"
        "collection"
      ];
  launcherToggle = [
    "qs"
    "ipc"
    "call"
    "launcher"
    "toggle"
  ];

  # The glyph each layer's shortcuts wear but the desktop's, whose actions
  # wear their own.
  layerGlyphs = {
    desktop = "";
    terminal = nerd "f489";
    multiplexer = nerd "f0bcc";
    program = nerd "f0614";
  };

  pieces = import ../theme/pieces.nix { inherit lib; };

  # The pieces Quickshell has built in, by name, each a file in pieces/.
  builtIn = {
    clock = ./pieces/Clock.qml;
    battery = ./pieces/Battery.qml;
    network = ./pieces/Network.qml;
    volume = ./pieces/Volume.qml;
    awake = ./pieces/Awake.qml;
    notices = ./pieces/Waiting.qml;
    quiet = ./pieces/DoNotDisturb.qml;
    microphone = ./pieces/Microphone.qml;
    capslock = ./pieces/CapsLock.qml;
    sharing = ./pieces/Sharing.qml;
    recording = ./pieces/Recording.qml;
    playing = ./pieces/Playing.qml;
  };

  overview = pieces.drawn config.jitsusama.quickshell.overview.pieces;

  # A built-in piece is linked by its file's name, and a machine's own by
  # the piece's name, so two of them never share a file.
  isBuiltIn = piece: builtins.isString piece.source && builtIn ? ${piece.source};
  fileName =
    piece: if isBuiltIn piece then baseNameOf builtIn.${piece.source} else "${piece.name}.qml";
  linked = piece: if isBuiltIn piece then builtIn.${piece.source} else piece.source;

  launcher = config.jitsusama.quickshell.launcher;

  # The kinds of thing the launcher has built in, by name, each a file in
  # launcher/.
  builtInProviders = {
    windows = ./launcher/Windows.qml;
    applications = ./launcher/Applications.qml;
    actions = ./launcher/Actions.qml;
    session = ./launcher/Session.qml;
    calculator = ./launcher/Calculator.qml;
    networks = ./launcher/Networks.qml;
    vendors = ./launcher/Vendors.qml;
    currency = ./launcher/Currency.qml;
    units = ./launcher/Units.qml;
    colours = ./launcher/Colours.qml;
    time = ./launcher/Clocks.qml;
    weather = ./launcher/Weather.qml;
    machine = ./launcher/Machine.qml;
    emoji = ./launcher/Emoji.qml;
    shortcuts = ./launcher/KeyMap.qml;
    clipboard = ./launcher/Copies.qml;
    notices = ./launcher/PastNotices.qml;
    wifi = ./launcher/Wireless.qml;
    sound = ./launcher/Sound.qml;
    bluetooth = ./launcher/Accessories.qml;
    displays = ./launcher/Displays.qml;
  };

  # The program that hears every copy, for the clipboard collection.
  clipboardHistory = pkgs.callPackage ./clipboard/package.nix { };
  keepsCopies = launcher.providers.clipboard.enable or false;

  # Every emoji, in a skin tone.
  emojiList = import ./launcher/emoji.nix { inherit lib pkgs; };
  timeZones = import ./launcher/zones.nix { inherit lib pkgs; };

  # The IEEE's hardware address vendors, from hwdata, in a file for each
  # first byte: a line for each vendor, its first three bytes and its name,
  # each after a line break, so one is found by what it starts with.
  vendors = pkgs.runCommandLocal "hardware-address-vendors" { } ''
    mkdir -p $out
    ${lib.getExe pkgs.gawk} -v out=$out '
      match($0, /^([0-9A-F]{6})[ \t]+\(base 16\)[ \t]+(.*[^ \t\r])/, m) {
        file = out "/" substr(m[1], 1, 2)
        if (!(file in seen)) { printf "\n" > file; seen[file] = 1 }
        printf "%s\t%s\n", m[1], m[2] > file
      }
    ' ${pkgs.hwdata}/share/hwdata/oui.txt
  '';

  providers = lib.sortOn (provider: provider.order) (
    lib.mapAttrsToList (name: provider: provider // { inherit name; }) (
      lib.filterAttrs (_: provider: provider.enable) launcher.providers
    )
  );

  # Named as the overview's pieces are, so two never share a file.
  isBuiltInProvider =
    provider: builtins.isString provider.source && builtInProviders ? ${provider.source};
  providerFile =
    provider:
    if isBuiltInProvider provider then
      baseNameOf builtInProviders.${provider.source}
    else
      "${provider.name}.qml";
  providerSource =
    provider:
    if isBuiltInProvider provider then builtInProviders.${provider.source} else provider.source;
in
{
  imports = [
    ../theme/home.nix
    ../keys/home.nix
  ];

  options.jitsusama.quickshell.overview.pieces = lib.mkOption {
    type = lib.types.attrsOf (
      pieces.piece {
        inherit (config.jitsusama.theme) glyphs;
        sources = builtIn;
      }
    );
    description = ''
      What the overview shows, by name. Each piece has its order, tier,
      collapse rank, mark and source (docs/design.md, Customising), and its
      own settings beside them: a `label` makes it a row, a muted label
      beside its value, and without one it's a heading above the rows, at
      the type size `size` names; a clock takes a `format`, as Qt spells
      one, and the battery `low` and `critical` percentages. A machine
      changes one setting in one line, or adds a piece whose source is a
      QML file starting from pieces/Piece.qml; lib.mkForce replaces them
      all.
    '';
  };

  options.jitsusama.quickshell.quiet.whileSharing = lib.mkOption {
    type = lib.types.bool;
    default = true;
    description = ''
      Whether do not disturb is on by itself while another program sees the
      screen, as a call shares it, and off again once nothing does. Ending it
      meanwhile lets notices show until the sharing stops.
    '';
  };

  options.jitsusama.quickshell.launcher = {
    providers = lib.mkOption {
      type = lib.types.attrsOf (
        lib.types.submodule {
          # A provider's own settings sit beside these and go to its file as
          # they are.
          freeformType = lib.types.attrsOf lib.types.anything;
          options = {
            enable = lib.mkOption {
              type = lib.types.bool;
              default = true;
              description = "Whether the launcher finds this kind of thing.";
            };
            order = lib.mkOption {
              type = lib.types.int;
              description = "Where it's made, lowest first; how well things match orders rows.";
            };
            weight = lib.mkOption {
              type = lib.types.number;
              default = 1;
              description = "How its matches rank against the rest's, multiplying their score.";
            };
            source = lib.mkOption {
              type = lib.types.either (lib.types.enum (lib.attrNames builtInProviders)) lib.types.path;
              description = "What finds them: a built-in provider by name, or a file like one.";
            };
          };
        }
      );
      description = ''
        What the launcher finds, by name: windows, applications, actions and
        the session, each with its order, weight and source, and settings of
        its own beside them, such as `glyphs`, an application's glyph by its
        id as a hex code point. A machine turns one off with `enable`, or
        adds a provider whose source is a QML file starting from
        launcher/Provider.qml.
      '';
    };
    lookups = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = ''
        Whether an answer may ask the network when what's typed asks for it:
        the weather, currency rates, public addresses and who holds one.
        Off, nothing the launcher does leaves the machine.
      '';
    };
    learning = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Whether what's picked for what was typed ranks it first next time.";
      };
      halfLife = lib.mkOption {
        type = lib.types.number;
        default = 14;
        description = "Days until a pick counts half as much, so old habits fade.";
      };
      remember = lib.mkOption {
        type = lib.types.ints.positive;
        default = 500;
        description = "How many queries are remembered; the least recently used go first.";
      };
    };
  };

  config = {
    # A thing found by name ranks a little ahead of an action matching as
    # well, and an action ahead of ending the session.
    jitsusama.quickshell.launcher.providers = lib.mapAttrs (_: lib.mapAttrs (_: lib.mkDefault)) {
      windows = {
        order = 10;
        source = "windows";
      };
      applications = {
        order = 20;
        source = "applications";
      };
      actions = {
        order = 30;
        source = "actions";
        weight = 0.9;
      };
      session = {
        order = 40;
        source = "session";
        weight = 0.8;
      };
      # Answers, asked in this order; the first to answer is the one shown.
      calculator = {
        order = 50;
        source = "calculator";
      };
      networks = {
        order = 60;
        source = "networks";
        inherit (launcher) lookups;
      };
      vendors = {
        order = 70;
        source = "vendors";
        directory = "${vendors}";
      };
      units = {
        order = 75;
        source = "units";
      };
      colours = {
        order = 78;
        source = "colours";
      };
      currency = {
        order = 80;
        source = "currency";
        inherit (launcher) lookups;
      };
      weather = {
        order = 90;
        source = "weather";
        units = "metric";
        zone = osConfig.time.timeZone or null;
        inherit (launcher) lookups;
      };
      time = {
        order = 95;
        source = "time";
        zone = osConfig.time.timeZone or null;
        inherit (launcher) lookups;
      };
      machine = {
        order = 100;
        source = "machine";
      };
      # Collections, each shown as one row below the rest. Wi-Fi's networks
      # are one, and turning Wi-Fi off is found as an action.
      wifi = {
        order = 102;
        source = "wifi";
        weight = 0.9;
      };
      # The devices sound goes out of and comes in through.
      sound = {
        order = 103;
        source = "sound";
        weight = 0.9;
      };
      # The headphones, mice and keyboards paired or around, and turning
      # Bluetooth off as an action.
      bluetooth = {
        order = 104;
        source = "bluetooth";
        weight = 0.9;
      };
      # Each display's scale, refresh rate, size and rotation, for the
      # session.
      displays = {
        order = 106;
        source = "displays";
        weight = 0.9;
      };
      clipboard = {
        order = 105;
        source = "clipboard";
        # How many copies are kept, in memory only.
        keep = 200;
      };
      emoji = {
        order = 110;
        source = "emoji";
        tone = "none";
      };
      notices = {
        order = 115;
        source = "notices";
      };
      # Opened by its own chord, not shown in a search.
      shortcuts = {
        order = 120;
        source = "shortcuts";
      };
    };

    # Each setting is a default, so a machine's own wins without a fight.
    jitsusama.quickshell.overview.pieces = lib.mapAttrs (_: lib.mapAttrs (_: lib.mkDefault)) {
      time = {
        order = 10;
        source = "clock";
        format = "HH:mm";
        size = "display";
      };
      date = {
        order = 20;
        source = "clock";
        format = "dddd d MMMM";
        collapse = 30;
      };
      battery = {
        order = 30;
        source = "battery";
        label = "battery";
        low = 20;
        critical = 10;
        collapse = 20;
        mark = "low";
      };
      network = {
        order = 40;
        source = "network";
        label = "network";
        collapse = 10;
      };
      volume = {
        order = 50;
        source = "volume";
        label = "volume";
        collapse = 0;
      };
      # A mode, so it never gives way.
      awake = {
        order = 60;
        source = "awake";
        label = "awake";
        tier = "when";
        mark = "awake";
      };
      # What's on only now and then, each also a pull in the margin while
      # it's true (Margin.qml), in the order the overview gives them.
      notices = {
        order = 54;
        source = "notices";
        label = "notices";
        tier = "when";
        mark = "waiting";
      };
      quiet = {
        order = 57;
        source = "quiet";
        label = "quiet";
        tier = "when";
        mark = "quiet";
      };
      microphone = {
        order = 70;
        source = "microphone";
        label = "mic";
        tier = "when";
        mark = "muted";
      };
      capslock = {
        order = 75;
        source = "capslock";
        label = "caps lock";
        tier = "when";
        mark = "capitals";
      };
      sharing = {
        order = 80;
        source = "sharing";
        label = "sharing";
        tier = "when";
        mark = "sharing";
      };
      recording = {
        order = 90;
        source = "recording";
        label = "recording";
        tier = "when";
        mark = "recording";
      };
      # The least of them, since music never needs Joel.
      playing = {
        order = 95;
        source = "playing";
        label = "playing";
        tier = "when";
        mark = "playing";
      };
    };

    home.packages = [ pkgs.quickshell ];

    xdg.configFile =
      lib.genAttrs (map (file: "quickshell/${file}") qml) (path: {
        source = ./. + "/${lib.removePrefix "quickshell/" path}";
      })
      // lib.listToAttrs (
        map (
          piece: lib.nameValuePair "quickshell/pieces/${fileName piece}" { source = linked piece; }
        ) overview
      )
      // lib.listToAttrs (
        map (
          provider:
          lib.nameValuePair "quickshell/launcher/${providerFile provider}" {
            source = providerSource provider;
          }
        ) providers
      )
      // {
        "quickshell/Providers.qml".text = ''
          pragma Singleton
          import Quickshell
          import QtQuick

          // What the launcher finds, in order, from
          // jitsusama.quickshell.launcher.providers, and how it learns, from
          // jitsusama.quickshell.launcher.learning.
          Singleton {
              readonly property var providers: ${
                builtins.toJSON (
                  map (
                    provider:
                    removeAttrs provider [
                      "source"
                      "order"
                      "enable"
                    ]
                    // {
                      file = providerFile provider;
                    }
                    # Emoji are listed in the tone chosen, so the list is
                    # built from the provider's own settings.
                    // lib.optionalAttrs (provider.source == "emoji") {
                      list = "${emojiList (provider.tone or "none")}";
                    }
                    // lib.optionalAttrs (provider.source == "time") {
                      list = "${timeZones}";
                    }
                    # A colour is told against the theme's own pigments.
                    // lib.optionalAttrs (provider.source == "colours") {
                      inherit roles;
                      inherit (config.jitsusama.theme) palette;
                    }
                  ) providers
                )
              }
              readonly property var learning: ${object launcher.learning}
          }
        '';
        "quickshell/Options.qml".text = ''
          pragma Singleton
          import Quickshell
          import QtQuick

          // The shell's own choices, from jitsusama.quickshell.
          Singleton {
              readonly property var quiet: ${object config.jitsusama.quickshell.quiet}
          }
        '';
        "quickshell/Overview.qml".text = ''
          pragma Singleton
          import Quickshell
          import QtQuick

          // The overview's pieces, in order, from jitsusama.quickshell.overview.pieces.
          Singleton {
              readonly property var pieces: ${
                builtins.toJSON (
                  map (
                    piece:
                    removeAttrs piece [
                      "source"
                      "order"
                    ]
                    // {
                      file = fileName piece;
                    }
                  ) overview
                )
              }
          }
        '';
        # Not Keys.qml, which would hide QtQuick's Keys from every file.
        "quickshell/Shortcuts.qml".text = ''
          pragma Singleton
          import Quickshell
          import QtQuick

          // The desktop's actions the launcher can find by name, from
          // jitsusama.keys: what each does, its chord drawn key by key, its
          // glyph and the command that does the same.
          Singleton {
              readonly property var actions: ${builtins.toJSON actions}
              // Every layer's shortcuts, top to bottom, each with its glyph.
              readonly property var layers: ${builtins.toJSON shortcutLayers}
          }
        '';
        "quickshell/Theme.qml".text = ''
          pragma Singleton
          import Quickshell
          import QtQuick

          Singleton {
          ${lib.concatLines (
            lib.mapAttrsToList (name: color: "    readonly property color ${name}: \"${color}\"") roles
          )}
              readonly property var mono: ${face font.mono.family type.mono}
              readonly property var sans: ${face font.sans.family type.sans}
              readonly property var space: ${object (removeAttrs space [ "unit" ])}
              readonly property int border: ${toString shape.border}
              readonly property int radius: ${toString shape.radius}
              readonly property int inset: ${toString shape.inset}
              // A part's corners: the whole's radius less its border and the
              // inset, so the curves run parallel.
              readonly property int inner: ${toString inner}
              readonly property var floating: ${
                object {
                  opacity = builtins.fromJSON floating.opacity;
                  scrim = builtins.fromJSON floating.scrim;
                  selection = builtins.fromJSON floating.selection;
                  # How far the shadows reach past the surface, which its
                  # window leaves room for.
                  reach = lib.foldl' lib.max 0 (map (s: s.offset + s.blur) floating.shadows);
                  shadows = map (s: {
                    inherit (s) offset blur;
                    opacity = builtins.fromJSON s.opacity;
                  }) floating.shadows;
                }
              }
              readonly property var motion: ${
                object {
                  # Slowed as niri slows its own, so the two stay in step.
                  inherit (lib.mapAttrs (_: slowed) { inherit (motion) short medium long; })
                    short
                    medium
                    long
                    ;
                  # Easing.BezierSpline's form: the two control points, then the end.
                  curve = map builtins.fromJSON motion.curve ++ [
                    1
                    1
                  ];
                }
              }
              readonly property var glyph: ${object (lib.mapAttrs (_: mark: mark.glyph) glyphs)}
              // Each picture's file, empty for none.
              readonly property var wallpaper: ${
                object {
                  desktop = picture wallpaper.desktop;
                  lock = picture wallpaper.lock;
                  shadows = map (s: {
                    inherit (s) x y blur;
                    opacity = builtins.fromJSON s.opacity;
                  }) wallpaper.shadows;
                  halo = map (s: {
                    inherit (s) blur;
                    opacity = builtins.fromJSON s.opacity;
                  }) wallpaper.halo;
                }
              }
          }
        '';
      };

    # Keeps the clipboard's text copies for the launcher, which can't hear
    # them itself, while the clipboard collection is on.
    systemd.user.services.clipboard-history = lib.mkIf keepsCopies {
      Unit = {
        Description = "The clipboard's history, kept in memory for the launcher";
        PartOf = [ "graphical-session.target" ];
        After = [ "graphical-session.target" ];
        Before = [ "quickshell.service" ];
      };
      Service = {
        ExecStart = lib.concatStringsSep " " [
          (lib.getExe clipboardHistory)
          "--socket %t/clipboard-history.sock"
          "--keep ${toString launcher.providers.clipboard.keep}"
        ];
        Restart = "on-failure";
        RestartSec = 1;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };

    # Keeping awake, from the launcher (Caffeinate.qml): one service holds
    # logind's lock against sleep and the other its lock against the lid,
    # which the shell takes only while there's power. Each leaves a file in
    # the runtime directory while it runs, for the shell to watch.
    systemd.user.services.caffeinate = holding {
      what = "sleep:idle";
      flag = "caffeinated";
      Description = "Keeps the machine from sleeping while work runs";
      PartOf = [ "graphical-session.target" ];
    };
    systemd.user.services.caffeinate-lid = holding {
      what = "handle-lid-switch";
      flag = "caffeinated-lid";
      Description = "Keeps the machine running with its lid shut, while there's power";
      PartOf = [ "caffeinate.service" ];
      After = [ "caffeinate.service" ];
      ConditionACPower = true;
    };

    systemd.user.services.quickshell = {
      Unit = {
        Description = "Quickshell, which draws the launcher, notifications and lock screen";
        PartOf = [ "graphical-session.target" ];
        After = [ "graphical-session.target" ];
        # Quickshell watches its files but not the links Home Manager swaps
        # them by, so a switch would leave it drawing the old ones. Naming
        # every file here changes the unit when any of them changes, and Home
        # Manager restarts a changed unit.
        X-Restart-Triggers = lib.mapAttrsToList (_: file: "${file.source}") (
          lib.filterAttrs (name: _: lib.hasPrefix "quickshell/" name) config.xdg.configFile
        );
      };
      Service = {
        ExecStart = lib.getExe' pkgs.quickshell "quickshell";
        Restart = "on-failure";
        RestartSec = 1;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };
  };
}
