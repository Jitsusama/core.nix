# Quickshell draws everything on Joel's screen that isn't a window: the
# wallpaper, the launcher on Super+Space, notifications, the lock screen,
# polkit's password prompt, SSH's askpass for a PIN or a yes, the volume,
# microphone, brightness and charge as they change, what's playing, and the
# time and status over the overview.
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
    "KeyHints.qml"
    "Launcher.qml"
    "Choice.qml"
    "Margin.qml"
    "Notifications.qml"
    "Music.qml"
    "Music.js"
    "Lock.qml"
    "Polkit.qml"
    "Askpass.qml"
    "Askpass.js"
    "Volume.qml"
    "Power.qml"
    "Brightness.qml"
    "Meter.qml"
    "Facts.qml"
    "Wallpaper.qml"
    "Lifted.qml"
    "OpenBelow.qml"
    "Niri.qml"
    "Wifi.qml"
    "SignIn.qml"
    "Floating.qml"
    "Fuzzy.js"
    "Picks.qml"
    "Notices.qml"
    "Caffeinate.qml"
    "Recorder.qml"
    "Picker.qml"
    "Picking.js"
    "Quiet.qml"
    "Lights.qml"
    "KeyboardLayer.qml"
    "KeyboardLayout.qml"
    "KeyboardMap.qml"
    "Report.qml"
    "Reported.qml"
    "pieces/Piece.qml"
    "launcher/Provider.qml"
    "launcher/Numbers.js"
    "launcher/Addresses.js"
    "launcher/Units.js"
    "launcher/Zones.js"
    "launcher/Colours.js"
    "launcher/Asking.js"
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
  # and a word starts with one. A sequence (space b w) is each of its chords
  # in turn, the first key of each after the first marked as coming after,
  # with no + before it.
  chordFaces =
    keys:
    lib.concatLists (
      lib.imap0 (
        index: step: lib.imap0 (at: face: face // { after = index > 0 && at == 0; }) (stepFaces step)
      ) (lib.splitString " " keys)
    );
  stepFaces =
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
  # does the same and the glyph its action wears. The touchscreen's swipes
  # come after the desktop's chords, since they do what its chords do.
  shortcutLayers =
    let
      keyed = layer: {
        name = layer;
        glyph = layerGlyphs.${layer};
        binds = map (b: {
          inherit (b) does run when;
          chord = chordFaces b.keys;
          glyph = if layer == "desktop" then actionGlyph b.run else "";
        }) config.jitsusama.keys.${layer};
      };
    in
    [
      (keyed "desktop")
      touchscreen
    ]
    ++ map keyed [
      "terminal"
      "neovim"
      "pi"
    ];

  # Each swipe drawn as a chord is: how many fingers, then which way, the
  # niri action it runs and the glyph that action wears.
  touchscreen = {
    name = "touchscreen";
    glyph = "";
    binds = map (
      swipe:
      let
        run = [
          "niri"
          "msg"
          "action"
          swipe.action
        ];
      in
      {
        inherit (swipe) does;
        inherit run;
        chord = [
          {
            text = toString swipe.fingers;
            wide = false;
          }
          {
            text = swipeGlyphs.${swipe.way};
            wide = true;
          }
        ];
        glyph = actionGlyph run;
      }
    ) config.jitsusama.swipes;
  };
  swipeGlyphs = {
    up = nerd "f0740";
    down = nerd "f073d";
    left = nerd "f073e";
    right = nerd "f073f";
  };

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
    # What the margin shows while the screen is recorded.
    else if program == "qs" && lib.elem "recording" run then
      config.jitsusama.theme.glyphs.recording.glyph
    # What the margin shows while the screen is shared.
    else if lib.hasInfix "dynamic-cast" action then
      config.jitsusama.theme.glyphs.sharing.glyph
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
        capture = nerd "f0100";
        clipboard = nerd "f0a38";
        emoji = nerd "f01f5";
        notices = nerd "f009c";
      }
      .${lib.last run} or (nerd "f097b")
    # The margin's drawer, which hangs from the top of the screen.
    else if program == "qs" && lib.elem "margin" run then
      nerd "f1513"
    else if program == "qs" && lib.elem "keyboard" run then
      nerd "f030c"
    else
      nerd "f2d0";

  # The desktop's binds the launcher can run, its own toggle left out, since
  # picking that from the launcher would only close it.
  # Each with the launcher's collection it opens, if it opens one, so the
  # launcher steps into it rather than closing to open again, and its home
  # screen shows the chord beside the collection.
  # One row per thing done: where two chords do the same, as Super+L and
  # the lock key do, the row shows the first.
  actions =
    map
      (b: {
        inherit (b) does run;
        glyph = actionGlyph b.run;
        chord = map face' (lib.splitString "+" b.chord);
        # Super+N takes the notices on screen, and opens their collection
        # when there are none, so the launcher steps into it.
        opens =
          if opensCollection b.run then
            lib.last b.run
          else if b.run == takesNotices then
            "notices"
          else
            "";
      })
      (
        firstOfEach (lib.filter (b: b.run != [ ] && b.run != launcherToggle) config.jitsusama.keys.desktop)
      );
  firstOfEach = lib.foldl' (
    kept: b: if lib.any (k: k.run == b.run) kept then kept else kept ++ [ b ]
  ) [ ];
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
  takesNotices = [
    "qs"
    "ipc"
    "call"
    "notices"
    "take"
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
    neovim = nerd "e6ae";
    pi = nerd "f03ff";
  };

  pieces = import ../theme/pieces.nix { inherit lib; };

  # The pieces Quickshell has built in, by name, each a file in pieces/.
  builtIn = {
    clock = ./pieces/Clock.qml;
    battery = ./pieces/Battery.qml;
    network = ./pieces/Network.qml;
    volume = ./pieces/Volume.qml;
    awake = ./pieces/Awake.qml;
    disk = ./pieces/Disk.qml;
    spend = ./pieces/Spend.qml;
    notices = ./pieces/Waiting.qml;
    quiet = ./pieces/DoNotDisturb.qml;
    microphone = ./pieces/Microphone.qml;
    capslock = ./pieces/CapsLock.qml;
    layer = ./pieces/Layer.qml;
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
    power = ./launcher/Power.qml;
    heat = ./launcher/Heat.qml;
    today = ./launcher/Today.qml;
    light = ./launcher/Light.qml;
    keyboards = ./launcher/Keyboards.qml;
    emoji = ./launcher/Emoji.qml;
    shortcuts = ./launcher/KeyMap.qml;
    keyboard = ./launcher/Keyboard.qml;
    clipboard = ./launcher/Copies.qml;
    notices = ./launcher/PastNotices.qml;
    wifi = ./launcher/Wireless.qml;
    sound = ./launcher/Sound.qml;
    bluetooth = ./launcher/Accessories.qml;
    displays = ./launcher/Displays.qml;
    capture = ./launcher/Capture.qml;
    spend = ./launcher/Spend.qml;
    agents = ./launcher/Agents.qml;
    byhand = ./launcher/ByHand.qml;
  };

  # Slack's theme in the ten colours its importer reads: the sidebar, a
  # row under the pointer, the chosen row and its text, a hovered row, the
  # sidebar's text, someone present, a mention's badge, and the top bar and
  # its text. Its frame sits a step deeper than its rows, as GTK's does.
  slackTheme = lib.concatMapStringsSep "," (role: roles.${role}) [
    "deepest"
    "raised"
    "selection"
    "strong"
    "raised"
    "text"
    "success"
    "alert"
    "deepest"
    "text"
  ];

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

  # OpenSSH's askpass, asked through the shell (Askpass.qml): the prompt is
  # the only argument, SSH_ASKPASS_PROMPT says whether it's a secret, a yes
  # ("confirm") or a notice that waits on a touch ("none"), the answer goes
  # to standard output, and a refusal exits nonzero. The shell answers into
  # a folder made for it, watched before asking so the answer can't come
  # unseen. A notice lasts until the program stops the askpass, and anything
  # that ends it early takes the prompt back down. The shell holds an ask only
  # in memory, so while one is up the askpass checks every few seconds that
  # the shell still has it: a shell that restarted is asked again, and once
  # no shell has answered for 20 seconds the askpass refuses. Otherwise the
  # program would wait on it forever, and ssh-tpm-agent every key with it.
  askpass = pkgs.writeShellApplication {
    name = "ssh-askpass";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.inotify-tools
      pkgs.quickshell
    ];
    text = ''
      kind=''${SSH_ASKPASS_PROMPT:-secret}
      # Who asked: the program that ran this, or the script an interpreter
      # runs, named without its store path or a wrapper's dots.
      argv=()
      mapfile -d "" -t argv < "/proc/$PPID/cmdline" || true
      asker=''${argv[0]:-}
      asker=''${asker##*/}
      case $asker in
        bash | sh | dash | zsh | python* | perl)
          script=''${argv[1]:-}
          [[ $script == -* || $script == "" ]] || asker=''${script##*/}
          ;;
      esac
      [[ $asker =~ ^[0-9a-z]{32}-(.+)$ ]] && asker=''${BASH_REMATCH[1]}
      [[ $asker =~ ^\.(.+)-wrapped$ ]] && asker=''${BASH_REMATCH[1]}
      reply=$(mktemp -d -p "$XDG_RUNTIME_DIR" askpass.XXXXXXXX)
      exec 3< <(exec inotifywait -m -e moved_to,close_write --format %f "$reply" 2>&1)
      watcher=$!
      settled() { [ -e "$reply/answer" ] || [ -e "$reply/refused" ]; }
      finish() {
        settled || qs ipc call askpass withdraw "$reply" > /dev/null 2>&1 || true
        kill "$watcher" 2> /dev/null || true
        rm -rf "$reply"
      }
      trap finish EXIT
      trap 'exit 1' TERM INT HUP
      while read -r line <&3 && [ "$line" != "Watches established." ]; do :; done

      ask() { qs ipc call askpass ask "$kind" "$asker" "$1" "$reply" > /dev/null; }
      if ! ask "$1"; then
        echo "ssh-askpass: the shell isn't there to ask" >&2
        exit 1
      fi
      every=3 unanswered=0
      until settled; do
        status=0
        read -r -t "$every" _ <&3 || status=$?
        # Beyond 128 is the wait ending; anything else is the watcher gone.
        [ "$status" -eq 0 ] && continue
        [ "$status" -gt 128 ] || exit 1
        if waiting=$(qs ipc call askpass waiting "$reply" 2> /dev/null); then
          unanswered=0
          # Answered just now, or forgotten by a shell that restarted.
          [ "$waiting" = true ] || settled || ask "$1" || true
        else
          unanswered=$((unanswered + every))
          if [ "$unanswered" -ge 20 ]; then
            echo "ssh-askpass: the shell went away without answering" >&2
            exit 1
          fi
        fi
      done
      [ -e "$reply/answer" ] || exit 1
      [ "$kind" = confirm ] || cat "$reply/answer"
    '';
  };
in
{
  imports = [
    ../theme/home.nix
    ../keys/home.nix
    ../camera/home.nix
    # The brightness bar shows the picks the backlight daemon tells it of.
    ../backlight/home.nix
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

  options.jitsusama.quickshell.reports = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = ''
      Whether the shell writes what it does to the journal, a line for each,
      such as a surface drawn or taking the keys, a notice held back, or the
      launcher's state. The desktop test turns it on and waits on these lines
      rather than guessing how long something takes.
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
      byhand = {
        order = 79;
        source = "byhand";
        programs = [
          {
            name = "Slack's Theme";
            app = "slack";
            keywords = "slack theme colours sidebar";
            side = "paste in Appearance, Import theme";
            copy = slackTheme;
          }
        ];
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
      # The battery, the charger and the power profile, in detail.
      power = {
        order = 99;
        source = "power";
      };
      # How hot each part runs, and the fans.
      heat = {
        order = 98;
        source = "heat";
      };
      # The date's week, day of the year, UTC, zone and epoch.
      today = {
        order = 97;
        source = "today";
        zone = osConfig.time.timeZone or null;
      };
      # How bright the screen and the keyboard are.
      light = {
        order = 96;
        source = "light";
      };
      # Which keyboards are plugged in, and caps lock.
      keyboards = {
        order = 94;
        source = "keyboards";
        # How a held key repeats, which every keyboard shares.
        repeat = import ../keyboard/repeat.nix;
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
      # Pictures and videos of the screen, a window, a region or from the
      # camera, which Print opens; the camera's are there when it has one.
      capture = {
        order = 101;
        source = "capture";
        weight = 0.9;
        camera = config.jitsusama.camera.device != null;
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
      # What AI use cost, where spend-watch runs (modules/spend-watch), and
      # nothing elsewhere; it answers "spend" and its reports.
      spend = {
        order = 108;
        source = "spend";
        weight = 0.9;
      };
      # The pi sessions running, each found by its quest.
      agents = {
        order = 109;
        source = "agents";
        weight = 0.9;
      };
      # Opened by its own chord, not shown in a search.
      shortcuts = {
        order = 120;
        source = "shortcuts";
      };
      # A keyboard's layout and what can be done to it, where a keyboard's
      # module describes one (KeyboardLayout.qml), and empty elsewhere.
      keyboard = {
        order = 107;
        source = "keyboard";
        weight = 0.9;
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
      # What AI use cost today against usual, where spend-watch runs
      # (modules/spend-watch), and a pull in the margin while something
      # wants a look. It gives way before the network.
      spend = {
        order = 52;
        source = "spend";
        label = "ai";
        collapse = 5;
        mark = "spend";
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
      # Only while a disk is short of room or nothing watches them: the
      # guard keeps them healthy without being asked.
      disk = {
        order = 58;
        source = "disk";
        label = "disks";
        tier = "when";
        mark = "disk";
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
      layer = {
        order = 77;
        source = "layer";
        label = "layer";
        tier = "when";
        mark = "layer";
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

    # The recorder's tools (Recorder.qml): wf-recorder records the screen
    # and record-window a window, grim takes a region's picture and wl-copy
    # copies it, and notify-send says where the file went.
    home.packages = [
      askpass
      pkgs.quickshell
      pkgs.wf-recorder
      (pkgs.callPackage ./record-window/package.nix { })
      pkgs.grim
      pkgs.wl-clipboard
      pkgs.libnotify
    ];

    # For shells and for the services systemd starts, ssh-tpm-agent among
    # them.
    home.sessionVariables.SSH_ASKPASS = lib.getExe askpass;
    systemd.user.sessionVariables.SSH_ASKPASS = lib.getExe askpass;

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
              readonly property bool reports: ${lib.boolToString config.jitsusama.quickshell.reports}
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

    # Keeps the clipboard's copies for the launcher, which can't hear them
    # itself, while the clipboard collection is on: text in memory, and
    # pictures in the runtime directory, which is memory too.
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
          "--pictures %t/clipboard-history"
        ];
        Restart = "on-failure";
        RestartSec = 1;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };

    # Keeping awake, from the launcher (Caffeinate.qml): one service holds
    # logind's lock against going idle and the other its lock against the
    # lid, which the shell takes only while there's power. Neither holds the
    # lock against sleep, which would refuse a suspend Joel asks for himself
    # with the power key or the launcher, and the lid's suspend on battery.
    # Each leaves a file in the runtime directory while it runs, for the
    # shell to watch.
    systemd.user.services.caffeinate = holding {
      what = "idle";
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
        # GTK's platform theme, which other Qt programs take, changes the
        # shell's default font and slows its start, and it draws from the
        # theme itself.
        UnsetEnvironment = [ "QT_QPA_PLATFORMTHEME" ];
        Restart = "on-failure";
        RestartSec = 1;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };
  };
}
