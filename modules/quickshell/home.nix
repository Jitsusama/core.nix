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
  # one, for the weather's home: the time zone, and whether GeoClue runs.
  osConfig ? { },
  ...
}:
let
  inherit (config.jitsusama.theme)
    colours
    font
    shape
    space
    type
    motion
    marks
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
  # The marks the theme draws as paths, for Theme.drawn.
  drawn = lib.mapAttrs (_: mark: mark.drawn) (lib.filterAttrs (_: mark: mark.drawn != "") marks);
  slowed = milliseconds: builtins.floor (milliseconds * builtins.fromJSON motion.slowdown + 0.5);
  face = family: sizes: object ({ inherit family; } // lib.mapAttrs (_: builtins.fromJSON) sizes);

  qml = [
    "shell.qml"
    "Prompt.qml"
    "KeyHints.qml"
    "Drawer.qml"
    "Launcher.qml"
    "Choice.qml"
    "Reading.js"
    "Graph.qml"
    "Chord.qml"
    "Margin.qml"
    "Marks.js"
    "Arrivals.qml"
    "Levels.js"
    "Crossing.js"
    "Devices.js"
    "Facts.js"
    "Stack.js"
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
    "ProcessorTelemetry.qml"
    "SignIn.qml"
    "Floating.qml"
    "Fuzzy.js"
    "Picks.qml"
    "Trials.qml"
    "Words.qml"
    "Files.qml"
    "Taken.qml"
    "Pasting.qml"
    "Notices.qml"
    "Forward.js"
    "Sessions.qml"
    "Location.qml"
    "WaitingOnYou.qml"
    "Waiting.js"
    "SpendWatch.qml"
    "Jobs.qml"
    "Caffeinate.qml"
    "Recorder.qml"
    "Picker.qml"
    "Picking.js"
    "Quiet.qml"
    "Lights.qml"
    "DeskLight.qml"
    "KeyboardLayer.qml"
    "KeyboardLayout.qml"
    "KeyboardMap.qml"
    "Report.qml"
    "Reported.qml"
    "Frames.qml"
    "Counted.qml"
    "pieces/Piece.qml"
    "pieces/StatusFile.qml"
    "launcher/Provider.qml"
    "launcher/Playing.qml"
    "launcher/Recents.qml"
    "launcher/Sensors.qml"
    "launcher/Machine.js"
    "launcher/Heat.js"
    "launcher/Power.js"
    "launcher/Traffic.js"
    "launcher/Cameras.js"
    "launcher/Firmware.js"
    "launcher/Interfaces.js"
    "launcher/Links.js"
    "launcher/Sounds.js"
    "launcher/Numbers.js"
    "launcher/Addresses.js"
    "launcher/Subnets.js"
    "launcher/Characters.js"
    "launcher/Units.js"
    "launcher/Zones.js"
    "launcher/Ago.js"
    "launcher/Colours.js"
    "launcher/Asking.js"
    "launcher/Catalogue.js"
    "launcher/Rank.js"
    "launcher/Standing.js"
    "launcher/Steady.js"
    "launcher/SessionRow.js"
    "launcher/Trials.js"
    "launcher/Emoji.js"
    "launcher/KeyGroups.js"
    "launcher/Keyboards.js"
    "launcher/Now.js"
    "launcher/Notice.js"
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
  # A mark from the theme, by its name.
  mark = name: marks.${name}.glyph;

  # How a chord's keys are drawn, the theme's (../theme/keys.nix).
  keyFaces = config.jitsusama.theme.keys;
  # Keys that send their modifiers themselves, as the Copilot key sends
  # Shift and Super with F23, so a chord of them is drawn as the one key
  # pressed.
  wholeKeys = [ "f23" ];
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
      pressed = if lib.elem key wholeKeys then [ key ] else held ++ [ key ];
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
    map face pressed;

  # Every layer's shortcuts, top to bottom, for the launcher's collection of
  # them: what each does, its chord, in its one spelling too, and the group
  # it names, if any, for launcher/KeyGroups.js to group it by, and for the
  # desktop's, the command that does the same and the glyph its action
  # wears. The touchscreen's swipes are the desktop's too, after its
  # chords: a layer of their own split one desktop in two (Joel, 2026-10-25:
  # "touch and desktop being separate does not make sense"), and five of the
  # six do what a chord does, so each joins that chord's row
  # (launcher/KeyGroups.js's merged).
  shortcutLayers =
    let
      keyed = layer: {
        name = layer;
        glyph = layerGlyphs.${layer};
        binds = map (b: {
          inherit (b)
            does
            run
            when
            keys
            group
            ;
          chord = chordFaces b.keys;
          glyph = if layer == "desktop" then actionGlyph b.run else "";
        }) config.jitsusama.keys.${layer};
      };
      desktop = keyed "desktop";
    in
    [
      (desktop // { binds = desktop.binds ++ swipes; })
    ]
    ++ map keyed [
      "terminal"
      "neovim"
      "pi"
      "btop"
      "swayimg"
      "inlyne"
      "zathura"
      "mpv"
      "chrome"
      "slack"
      "onepassword"
      "gimp"
      "inkscape"
      "spotify"
    ];

  # Each swipe as a desktop bind: drawn as how many fingers, then which
  # way, and written as words ("3 fingers up") where a chord is spelled; the
  # niri action it runs, as the desktop's own binds run theirs; and the
  # keys and group of the desktop bind that does the same, so it lands in
  # that bind's group and row.
  swipes = map (
    swipe:
    let
      run = [
        "niri"
        "msg"
        "action"
        swipe.action
        "--"
      ];
      twin = lib.findFirst (b: b.does == swipe.does) null config.jitsusama.keys.desktop;
    in
    {
      inherit (swipe) does;
      inherit run;
      when = "";
      keys = if twin == null then "" else twin.keys;
      group = if twin == null then "" else twin.group;
      swipe = true;
      chord = [
        {
          text = toString swipe.fingers;
          wide = false;
          written = "${toString swipe.fingers} fingers ${swipe.way}";
        }
        {
          text = swipeGlyphs.${swipe.way};
          wide = true;
        }
      ];
      glyph = actionGlyph run;
    }
  ) config.jitsusama.swipes;
  swipeGlyphs = {
    up = mark "swipeUp";
    down = mark "swipeDown";
    left = mark "swipeLeft";
    right = mark "swipeRight";
  };

  # The glyph an action wears in the launcher, by what it runs: the theme's
  # mark for what it acts on, so a chord wears its group's mark in
  # Shortcuts (Windows, Columns, Workspaces) and a screenshot the camera
  # Capture's own rows wear. The terminal wears its program's logo, as
  # programs do.
  actionGlyph =
    run:
    let
      program = if run == [ ] then "" else lib.head run;
      action = if program == "niri" then lib.elemAt run 3 else "";
      says = lib.concatStringsSep " " run;
      asks = word: program == "qs" && lib.elem word run;
    in
    if lib.hasInfix "kitty" says then
      nerd "f489"
    else if lib.hasPrefix "screenshot" action then
      mark "capture"
    # What the margin shows while the screen is recorded.
    else if asks "recording" then
      mark "recording"
    # What the margin shows while the screen is shared.
    else if lib.hasInfix "dynamic-cast" action then
      mark "sharing"
    else if lib.hasInfix "AUDIO_SOURCE" says then
      mark "muted"
    else if program == "sh" && lib.hasInfix "wpctl" says then
      mark "volume"
    else if program == "brightnessctl" then
      mark "brightness"
    else if asks "music" then
      mark "playing"
    else if lib.hasInfix "workspace" action || lib.hasInfix "overview" action then
      mark "workspace"
    else if lib.hasInfix "monitor" action then
      mark "display"
    # A collection's own glyph, as its row in the launcher wears
    # (launcher/words.json).
    else if program == "launcher-say" && lib.elem "collection" run then
      let
        named = lib.findFirst (thing: thing.id == lib.last run) { } launcher.words.things;
      in
      mark (named.glyph or "shortcuts")
    else if program == "launcher-say" then
      mark "launcher"
    else if asks "keyboard" || lib.hasInfix "keyboard-shortcuts" action then
      mark "keyboard"
    else if program == "dictation" then
      mark "dictating"
    else if asks "lock" then
      mark "session"
    else if asks "quiet" then
      mark "quiet"
    else if asks "notices" && lib.elem "putAway" run then
      mark "dismiss"
    else if asks "notices" then
      mark "notifications"
    else if action == "close-window" then
      mark "close"
    else if lib.hasInfix "floating" action then
      mark "floating"
    else if lib.hasInfix "fullscreen" action then
      mark "fullscreen"
    else if lib.hasInfix "column" action then
      mark "columns"
    else
      mark "windows";

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
        inherit (b) does run also;
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
    lib.length run == 3
    &&
      lib.take 2 run == [
        "launcher-say"
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
    "launcher-say"
    "toggle"
  ];

  # The glyph each layer's shortcuts wear but the desktop's, whose actions
  # wear their own.
  layerGlyphs = {
    desktop = "";
    terminal = nerd "f489";
    neovim = nerd "e6ae";
    pi = nerd "f03ff";
    # The theme's processor, as btop watches it.
    btop = nerd "f061a";
    # The theme's picture, as the viewer shows pictures.
    swayimg = nerd "f02e9";
    # The theme's document, as the reader shows documents.
    inlyne = nerd "f0219";
    # The theme's book, as the viewer shows documents of pages.
    zathura = nerd "f05da";
    # The theme's film, as the player plays films.
    mpv = nerd "f0381";
    # The theme's web, as the browser goes about it.
    chrome = nerd "f059f";
    # The theme's chat, as Slack is where people talk.
    slack = nerd "f0b79";
    # The theme's password, as 1Password keeps them.
    onepassword = nerd "f0306";
    # The theme's brush, as GIMP paints on pictures.
    gimp = nerd "f00e3";
    # The theme's vector, as Inkscape draws in shapes and paths.
    inkscape = nerd "f0ae7";
    # The theme's music playing, as Spotify plays it.
    spotify = nerd "f075a";
  };

  pieces = import ../theme/pieces.nix { inherit lib; };

  # The pieces Quickshell has built in, by name, each a file in pieces/.
  builtIn = {
    clock = ./pieces/Clock.qml;
    battery = ./pieces/Battery.qml;
    devices = ./pieces/Devices.qml;
    network = ./pieces/Network.qml;
    volume = ./pieces/Volume.qml;
    awake = ./pieces/Awake.qml;
    profile = ./pieces/Profile.qml;
    disk = ./pieces/Disk.qml;
    lag = ./pieces/Lag.qml;
    spend = ./pieces/Spend.qml;
    notices = ./pieces/Waiting.qml;
    quiet = ./pieces/DoNotDisturb.qml;
    microphone = ./pieces/Microphone.qml;
    capslock = ./pieces/CapsLock.qml;
    layer = ./pieces/Layer.qml;
    sharing = ./pieces/Sharing.qml;
    camera = ./pieces/Camera.qml;
    pi = ./pieces/Pi.qml;
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
    characters = ./launcher/Characters.qml;
    colours = ./launcher/Colours.qml;
    time = ./launcher/Clocks.qml;
    weather = ./launcher/Weather.qml;
    ask = ./launcher/Ask.qml;
    machine = ./launcher/Machine.qml;
    power = ./launcher/Power.qml;
    heat = ./launcher/Heat.qml;
    traffic = ./launcher/Traffic.qml;
    waits = ./launcher/Waits.qml;
    firmware = ./launcher/Firmware.qml;
    today = ./launcher/Today.qml;
    light = ./launcher/Light.qml;
    keyboards = ./launcher/Keyboards.qml;
    emoji = ./launcher/Emoji.qml;
    shortcuts = ./launcher/KeyMap.qml;
    keyboard = ./launcher/Keyboard.qml;
    clipboard = ./launcher/Copies.qml;
    notices = ./launcher/PastNotices.qml;
    on = ./launcher/WhatsOn.qml;
    wifi = ./launcher/Wireless.qml;
    sound = ./launcher/Sound.qml;
    bluetooth = ./launcher/Accessories.qml;
    displays = ./launcher/Displays.qml;
    capture = ./launcher/Capture.qml;
    cameras = ./launcher/Cameras.qml;
    spend = ./launcher/Spend.qml;
    agents = ./launcher/Agents.qml;
    orchestrators = ./launcher/Orchestrators.qml;
    quests = ./launcher/Quests.qml;
    queue = ./launcher/Queue.qml;
    runs = ./launcher/Runs.qml;
    byhand = ./launcher/ByHand.qml;
  };

  # Slack's theme in the four colours its importer keeps: its frame, the
  # open channel, someone present and a mention's badge. Slack shades the
  # frame into its rail and sidebar and draws its own text, so the frame is
  # the accent, which Slack darkens into the theme's greens, rather than the
  # deepest background, which it shades to grey. A badge is the warning
  # colour, since Slack writes its number dark on that and light on the
  # alert colour, too faint to read (2.96:1).
  slackTheme = lib.concatMapStringsSep "," (role: colours.${role}) [
    "accent"
    "selection"
    "success"
    "warning"
  ];

  # The program that hears every copy, for the clipboard collection.
  clipboardHistory = pkgs.callPackage ./clipboard/package.nix { };
  keepsCopies = launcher.providers.clipboard.enable or false;

  # The program that says where the laptop is, for the weather's here. It
  # runs only while the weather may ask the network, since it has GeoClue
  # ask BeaconDB, and where the machine runs GeoClue (nixos.nix).
  whereabouts = pkgs.callPackage ./whereabouts/package.nix { };
  clockFollows = pkgs.callPackage ./clock-follows/package.nix { };
  timedatectl = lib.getExe' pkgs.systemd "timedatectl";
  # The program that says how the clock is kept, for Time's insides, where
  # the machine keeps it with systemd-timesyncd.
  timekeeping = pkgs.callPackage ./timekeeping/package.nix { };
  keepsTime = osConfig.services.timesyncd.enable or false;
  followsTheLaptop =
    (launcher.providers.weather.enable or false)
    && (launcher.providers.weather.lookups or launcher.lookups)
    && (osConfig.services.geoclue2.enable or false);

  # Every emoji, in a skin tone.
  emojiList = import ./launcher/emoji.nix { inherit lib pkgs; };
  timeZones = import ./launcher/zones.nix { inherit lib pkgs; };
  characterNames = import ./launcher/names.nix { inherit lib pkgs; };

  # Every process's counters at one moment, for the machine's answers to
  # say which programs use the processors and the memory: the page size,
  # then the machine's counters and each process's, in one spawn rather
  # than a read for each of hundreds of files on the shell's thread.
  processList = pkgs.writeShellScript "process-list" ''
    ${lib.getExe' pkgs.getconf "getconf"} PAGESIZE
    exec ${lib.getExe' pkgs.coreutils "cat"} /proc/stat /proc/[0-9]*/stat 2>/dev/null
  '';

  # A Wi-Fi card's link as iw reads it, for a network's detail in the
  # launcher: the link, the access point as the card hears it, and what
  # the card sends at, each headed so the launcher reads them apart, in one
  # spawn every few seconds while the network is shown.
  wifiLink = pkgs.writeShellScript "wifi-link" ''
    iw=${lib.getExe pkgs.iw}
    echo "@@ link"; "$iw" dev "$1" link
    echo "@@ station"; "$iw" dev "$1" station dump
    echo "@@ info"; "$iw" dev "$1" info
  '';

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

  # What a key says to the launcher: "toggle", or "collection" and a
  # collection's name. It goes down the socket the shell keeps open
  # (Launcher.qml), which takes a few milliseconds where starting `qs ipc`
  # takes a hundred or more, and every one of those is a letter typed
  # after Super+Space that could land in the window below. With no shell
  # listening, as while it restarts, qs ipc carries it instead.
  launcherSay = pkgs.writeShellApplication {
    name = "launcher-say";
    runtimeInputs = [
      pkgs.quickshell
      pkgs.socat
    ];
    text = ''
      printf '%s\n' "$*" | socat - "UNIX-CONNECT:$XDG_RUNTIME_DIR/launcher.sock" 2>/dev/null \
        || exec qs ipc call launcher "$@"
    '';
  };

  # A pi session that has come to want Joel, said as a notice from pi
  # (Sessions.qml starts it once a want): its quest and what it wants, and
  # Return or a click on it goes to the session as the launcher's Go To
  # does, through the shell, so one on another machine is joined here. A
  # notice's action reaches only a sender still waiting, so this waits
  # until the notice closes, which the shell does when the want ends, and
  # no more than an hour, so one a restarted shell forgot can't linger. Its
  # want's key goes with it as a hint, for the shell to know which notice
  # is whose. qs finds the shell by the display it draws on.
  piWants = pkgs.writeShellApplication {
    name = "pi-wants-you";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.libnotify
      pkgs.quickshell
    ];
    text = ''
      address=$1
      want=$2
      title=$3
      body=$4
      action=$(timeout 1h notify-send --app-name=pi --action=default="Go To" \
        --hint=string:x-pi-want:"$want" --wait -- "$title" "$body" || true)
      if [ "$action" = default ]; then
        qs ipc call sessions goTo "$address"
      fi
    '';
  };

  # OpenSSH's askpass, asked through the shell (./askpass.sh).
  askpass = pkgs.writeShellApplication {
    name = "ssh-askpass";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.gnused
      pkgs.inotify-tools
      pkgs.quickshell
      pkgs.systemd
    ];
    text = builtins.readFile ./askpass.sh;
  };
in
{
  imports = [
    ../theme/home.nix
    ../keys/home.nix
    ./recents.nix
    ../camera/home.nix
    # The brightness bar shows the picks the backlight daemon tells it of.
    ../backlight/home.nix
  ];

  options.jitsusama.quickshell.overview.pieces = lib.mkOption {
    type = lib.types.attrsOf (
      pieces.piece {
        inherit (config.jitsusama.theme) marks;
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

  options.jitsusama.quickshell.margin.linger = lib.mkOption {
    type = lib.types.ints.unsigned;
    default = 15;
    description = ''
      How many seconds something on air (a camera, a share, a recording, a
      call) stays in the margin, muted, once it has gone, so a camera let go
      is still seen to have been on. Nothing else lingers.
    '';
  };

  options.jitsusama.quickshell.notices.away = lib.mkOption {
    type = lib.types.ints.positive;
    default = 30;
    description = ''
      How many seconds with nothing typed or moved count as Joel being away,
      while a notice on screen waits rather than counting down, so one that
      comes while he's gone is there when he's back.
    '';
  };

  options.jitsusama.quickshell.notices.senders = lib.mkOption {
    type = lib.types.attrsOf (
      lib.types.submodule {
        options.level = lib.mkOption {
          type = lib.types.enum [
            "history"
            "count"
            "card"
            "urgent"
          ];
          description = ''
            The furthest a sender's notices reach: kept in the history alone,
            counted as waiting in the margin but never shown, shown as a card,
            or, when the notice is critical, shown as one that stays until
            it's answered. A notice reaches less far when it asks for less,
            never further than this.
          '';
        };
      }
    );
    default = { };
    example = {
      Spotify.level = "history";
      Slack.level = "card";
    };
    description = ''
      How far each program's notices reach, by the name a notice gives as
      its sender, in any case. A program not named reaches as far as it
      asks.
    '';
  };

  options.jitsusama.quickshell.sounds.wants = lib.mkOption {
    type = lib.types.nullOr lib.types.path;
    default = "${pkgs.sound-theme-freedesktop}/share/sounds/freedesktop/stereo/message.oga";
    defaultText = lib.literalExpression ''"${pkgs.sound-theme-freedesktop}/share/sounds/freedesktop/stereo/message.oga"'';
    description = ''
      The one sound the shell makes, softly, when a pi session first wants
      Joel and its notice is said, so he hears it from across the room;
      never while notices are held, nor for a want he's already looking
      at. Null makes none.
    '';
  };

  options.jitsusama.quickshell.surfaces = lib.mkOption {
    type = lib.types.attrsOf lib.types.path;
    default = { };
    description = ''
      Surfaces another module adds to the shell, by a name starting with a
      capital, each a QML file whose root is a window or a Scope holding
      them, as dictation's own look at the bottom centre is. The shell
      loads each beside its own, as surfaces/<name>.qml.
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
    synonyms = lib.mkOption {
      type = lib.types.attrsOf (lib.types.listOf lib.types.str);
      default = { };
      example = {
        "session:Suspend" = [ "zzz" ];
        "program:com.onepassword.OnePassword" = [ "vault" ];
      };
      description = ''
        Other words that find a row, by the key it's learned by (the
        launcher's picks file names them). They find it, never name it, and
        the row shows as it always does. A name the row once had belongs in
        the vocabulary's `once` instead, which says so below it.
      '';
    };
    words = lib.mkOption {
      type = lib.types.attrsOf lib.types.anything;
      default = lib.importJSON ./launcher/words.json;
      defaultText = lib.literalExpression "lib.importJSON ./launcher/words.json";
      description = ''
        The launcher's vocabulary, as launcher/words.json writes it: its
        categories in order and what each holds, every thing's name as
        shown, the other words and old names that find it and the glyph it
        wears, the commands that take something after them, and the words
        meant to find more than one thing. A machine changes a name here,
        and the check that no word is ambiguous reads the file.
      '';
    };
    kinds = lib.mkOption {
      type = lib.types.attrsOf lib.types.str;
      default = {
        program = "program";
        window = "window";
        # Agents' own rows are its sessions (docs/glossary.md).
        agent = "session";
        quest = "quest";
        subagent = "subagent";
        orchestrator = "orchestrator";
        copy = "copy";
        notice = "notice";
        emoji = "emoji";
        key = "key";
        "Wi-Fi" = "network";
        Bluetooth = "device";
        sound = "device";
        display = "screen";
        camera = "camera";
        person = "person";
        call = "call";
      };
      description = ''
        The word a thing's row says at its right edge, by what its provider
        calls its things. A provider not named here is an act or a set,
        whose words say what it is, so its rows keep their chords there.
      '';
    };
    learning = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = ''
          Whether what's picked for what was typed reorders what those letters
          find. A row comes first for them, and for the longer words they
          start, once it's been picked there two more times than anything
          else, and keeps the place until another leads it by two, so one
          slip teaches nothing. A name typed whole still comes first, unless
          something else was picked for exactly those letters twice. It only
          orders: every match still shows. Off, nothing is kept or leaned on.
        '';
      };
      halfLife = lib.mkOption {
        type = lib.types.number;
        default = 14;
        description = ''
          Days until a pick counts half as much in ordering the rest of a
          list, so old habits fade; who leads by two counts every pick, never
          fading. It's also how long a row that's gone, such as a closed
          window or a dismissed notice, keeps its picks: gone and unpicked for
          longer, they're forgotten.
        '';
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
        # What each program had open lately, for its Recent
        # (launcher/Recents.qml), as each program's module says (recents.nix).
        inherit (config.jitsusama.quickshell.launcher) recents;
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
      characters = {
        order = 77;
        source = "characters";
      };
      colours = {
        order = 78;
        source = "colours";
      };
      byhand = {
        order = 79;
        source = "byhand";
        # Each named, and found, as the vocabulary says its id
        # (`byhand:<app>`): a part of its program.
        programs = [
          {
            app = "slack";
            side = "pick Aubergine, then Custom theme, Import";
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
      # What the network carries now, what the line can, and how quickly it
      # answers, from the kernel's counters and the shaper's measures.
      traffic = {
        order = 92;
        source = "traffic";
      };
      # Whether anything Joel uses waited, from lag-watch, which a machine
      # without it simply doesn't answer.
      waits = {
        order = 93;
        source = "waits";
      };
      # Each device's firmware and any update offered for it, from fwupd,
      # only where the system runs it, so a machine without it names no
      # Firmware in Machine.
      firmware = {
        order = 91;
        source = "firmware";
        enable = osConfig.services.fwupd.enable or false;
        fwupdmgr =
          if osConfig.services.fwupd.enable or false then
            lib.getExe' osConfig.services.fwupd.package "fwupdmgr"
          else
            "";
      };
      # The date's week, day of the year, UTC, zone and epoch.
      today = {
        order = 97;
        source = "today";
        zone = osConfig.time.timeZone or null;
      };
      # How the desk light is lit; the screen's brightness is the screen's
      # own, and the keyboard's lights are the keyboard's.
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
        # What reads a card's ports, and what moves one onto another.
        pwDump = lib.getExe' pkgs.pipewire "pw-dump";
        wpctl = lib.getExe' pkgs.wireplumber "wpctl";
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
      # The orchestrators, each talked to, paused or stopped through its
      # session's steer inbox.
      orchestrators = {
        order = 110;
        source = "orchestrators";
        weight = 0.9;
      };
      # pi's quests, found by their titles and ids, which go to the session
      # on one or open pi on it.
      quests = {
        order = 111;
        source = "quests";
        weight = 0.9;
      };
      # The machine's queue: the heavy work running and waiting its turn,
      # each job going to the session it's for.
      jobs = {
        order = 111;
        source = "queue";
        weight = 0.9;
      };
      # pi's fleets and review rounds of the last day, and those running,
      # each going to the session that started it, its participants'
      # answers read in a terminal.
      runs = {
        order = 112;
        source = "runs";
        weight = 0.9;
      };
      # The margin's glyphs in words, opened by Super+O or a glyph's click,
      # not shown in a search.
      on = {
        order = 113;
        source = "on";
      };
      # Each camera with its parts, and whether one is in use. Return opens
      # the viewfinder on the camera the machine calls camera, the one
      # pictures are taken with; the loopbacks' cards find their nodes in
      # sysfs, so what they give is read without opening them.
      cameras = {
        order = 114;
        source = "cameras";
        weight = 0.9;
        viewfinder = config.jitsusama.camera.device;
        cards = lib.mapAttrs (_: holder: holder.card) (osConfig.jitsusama.v4l2loopback.holders or { });
      };
      # Opened by its own chord, not shown in a search.
      shortcuts = {
        order = 120;
        source = "shortcuts";
      };
      # A quick question to a model, asked last so any other answer comes
      # first, through the pi module's pi-ask, which picks the machine's
      # plan, where the account has the pi module; and carried on in pi.
      ask = {
        order = 130;
        source = "ask";
        enable = lib.hasAttrByPath [ "pi" "ask" ] config.jitsusama;
        command = "pi-ask";
        session = "pi";
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
        format = "ddd d MMM";
        # Early, after the volume: the time, the battery, the network, the
        # agents and the day's spend are what Joel looks to the launcher's
        # foot for (2026-10-24), and the date is a word away.
        collapse = 3;
      };
      battery = {
        order = 30;
        source = "battery";
        label = "battery";
        sign = "charge";
        low = 20;
        critical = 10;
        collapse = 20;
        mark = "low";
      };
      # Each Bluetooth device running low, only while one is, by the
      # laptop battery's own marks, so one rule holds for every battery.
      devices = {
        order = 31;
        source = "devices";
        label = "devices";
        tier = "when";
        low = 20;
        critical = 10;
        mark = "bluetooth";
      };
      network = {
        order = 40;
        source = "network";
        label = "network";
        sign = "wireless";
        collapse = 10;
      };
      volume = {
        order = 50;
        source = "volume";
        label = "volume";
        sign = "volume";
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
      # A mode too, only while the profile isn't the one the machine chose
      # for the charger and the battery, its mark drawn with the profile's
      # own glyph.
      profile = {
        order = 61;
        source = "profile";
        label = "power";
        tier = "when";
        mark = "saver";
      };
      # What AI use cost today against usual, where spend-watch runs
      # (modules/spend-watch), and a glyph in the margin while something
      # wants a look. It gives way before the network. It wears the
      # agents' glyph, since a dollar sign before a dollar figure said
      # nothing of what was spent on (Joel, 2026-10-24), and the launcher's
      # foot draws it once for the two.
      spend = {
        order = 52;
        source = "spend";
        label = "ai";
        sign = "agent";
        collapse = 5;
        mark = "spend";
      };
      # How many pi sessions run and how many are working. It gives way
      # after the volume and the day's spend, which have homes elsewhere,
      # and before the network.
      pi = {
        order = 53;
        source = "pi";
        label = "pi";
        sign = "agent";
        collapse = 6;
        # In the margin while a session wants Joel.
        mark = "agent";
      };
      # What's on only now and then, each also a glyph in the margin while
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
      # Only once the same thing has kept the desktop or its apps waiting a
      # minute, from lag-watch; a moment's wait Joel feels for himself.
      lag = {
        order = 59;
        source = "lag";
        label = "lag";
        tier = "when";
        mark = "lag";
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
      # A program with a camera open, where the machine says which.
      camera = {
        order = 85;
        source = "camera";
        label = "camera";
        tier = "when";
        mark = "camera";
        # The cameras whose holders the machine names, as v4l2loopback
        # writes them under /run/camera-holders.
        holders = lib.attrNames (osConfig.jitsusama.v4l2loopback.holders or { });
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
    # copies it, and notify-send says where the file went. The launcher's
    # quests and runs read pi through pi-quests and pi-runs
    # (launcher/Quests.qml, launcher/Runs.qml), and a terminal shows a
    # run's answer through pi-runs too. The clipboard's Paste presses its
    # window's paste keys through wtype (launcher/Copies.qml).
    home.packages = lib.optional followsTheLaptop whereabouts ++ [
      askpass
      launcherSay
      piWants
      pkgs.quickshell
      pkgs.wf-recorder
      (pkgs.callPackage ./record-window/package.nix { })
      (pkgs.callPackage ./pi-helper.nix {
        name = "pi-quests";
        source = ./quests.rs;
      })
      (pkgs.callPackage ./pi-helper.nix {
        name = "pi-runs";
        source = ./runs.rs;
      })
      pkgs.grim
      pkgs.wl-clipboard
      pkgs.wtype
      pkgs.libnotify
    ];

    # For shells and for the services systemd starts, ssh-tpm-agent among
    # them.
    home.sessionVariables.SSH_ASKPASS = lib.getExe askpass;
    systemd.user.sessionVariables.SSH_ASKPASS = lib.getExe askpass;

    # run0 waits for polkit's prompt inside one D-Bus call, which systemd
    # gives up on after 25 seconds, cancelling a prompt Joel hadn't reached
    # yet (twice on 2026-10-24). A shell's calls wait two minutes instead;
    # services keep systemd's own limit.
    home.sessionVariables.SYSTEMD_BUS_TIMEOUT = "2min";

    xdg.configFile =
      lib.genAttrs (map (file: "quickshell/${file}") qml) (path: {
        source = ./. + "/${lib.removePrefix "quickshell/" path}";
      })
      // lib.listToAttrs (
        map (
          piece: lib.nameValuePair "quickshell/pieces/${fileName piece}" { source = linked piece; }
        ) overview
      )
      // lib.mapAttrs' (
        name: source: lib.nameValuePair "quickshell/surfaces/${name}.qml" { inherit source; }
      ) config.jitsusama.quickshell.surfaces
      // lib.listToAttrs (
        map (
          provider:
          lib.nameValuePair "quickshell/launcher/${providerFile provider}" {
            source = providerSource provider;
          }
        ) providers
      )
      // {
        # Read by Words.qml as it's written, so a name is said once.
        "quickshell/launcher/words.json".text = builtins.toJSON launcher.words;
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
                    // lib.optionalAttrs (provider.source == "characters") {
                      names = "${characterNames}";
                    }
                    # What the machine's answers spawn: the processes'
                    # counters, UDisks for the disk's health, and stat for
                    # a file system's room where no disk guard says it.
                    // lib.optionalAttrs (provider.source == "machine") {
                      processes = "${processList}";
                      busctl = lib.getExe' pkgs.systemd "busctl";
                      stat = lib.getExe' pkgs.coreutils "stat";
                    }
                    # What each connection is, from ip, and a Wi-Fi link
                    # in detail, from iw.
                    // lib.optionalAttrs (provider.source == "networks") {
                      ip = lib.getExe' pkgs.iproute2 "ip";
                    }
                    // lib.optionalAttrs (provider.source == "wifi") {
                      iw = "${wifiLink}";
                    }
                    # A colour is told against the theme's own pigments.
                    // lib.optionalAttrs (provider.source == "colours") {
                      roles = colours;
                      inherit (config.jitsusama.theme) palette;
                    }
                  ) providers
                )
              }
              readonly property var learning: ${object launcher.learning}
              readonly property var launcher: ${object { inherit (launcher) synonyms kinds; }}
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
              readonly property var margin: ${object config.jitsusama.quickshell.margin}
              // The sound a want makes, and what plays it; "" for none.
              readonly property var sounds: ${
                object {
                  wants = toString config.jitsusama.quickshell.sounds.wants;
                  player = lib.getExe' pkgs.pipewire "pw-play";
                }
              }
              readonly property var notices: ${
                object {
                  inherit (config.jitsusama.quickshell.notices) away;
                  senders = lib.mapAttrs' (
                    name: sender: lib.nameValuePair (lib.toLower name) sender.level
                  ) config.jitsusama.quickshell.notices.senders;
                }
              }
              // Other modules' surfaces, by file, from jitsusama.quickshell.surfaces.
              readonly property var surfaces: ${
                builtins.toJSON (map (name: "${name}.qml") (lib.attrNames config.jitsusama.quickshell.surfaces))
              }
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
            lib.mapAttrsToList (name: color: "    readonly property color ${name}: \"${color}\"") colours
          )}
              readonly property var mono: ${face font.mono.family type.mono}
              readonly property var sans: ${face font.sans.family type.sans}
              readonly property var space: ${object (removeAttrs space [ "unit" ])}
              readonly property int border: ${toString shape.border}
              readonly property int radius: ${toString shape.radius}
              readonly property int inset: ${toString shape.inset}
              // The gap niri leaves between windows and round the screen's
              // edge, which the margin's glyphs stand in.
              readonly property int gap: ${toString shape.gap}
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
                  # How much slower than written everything moves, for motion
                  # timed by its distance rather than by a duration, as a
                  # long row's scroll is.
                  slowdown = builtins.fromJSON motion.slowdown;
                  # Easing.BezierSpline's form: the two control points, then the end.
                  curve = map builtins.fromJSON motion.curve ++ [
                    1
                    1
                  ];
                }
              }
              readonly property var glyph: ${object (lib.mapAttrs (_: mark: mark.glyph) marks)}
              // The marks drawn as paths, for where a glyph is too small to
              // read: on a 12 by 12 grid, filled even-odd.
              readonly property var drawn: ${object drawn}
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
                  # Centred on the mark, in the shape Lifted reads.
                  marks = map (s: {
                    inherit (s) blur;
                    x = 0;
                    y = 0;
                    opacity = builtins.fromJSON s.opacity;
                  }) wallpaper.marks;
                }
              }
          }
        '';
      };

    # Keeps the clipboard's copies for the launcher, which can't hear them
    # itself, while the clipboard collection is on: text in memory, and
    # pictures in the runtime directory, which is memory too.
    # pi makes its quests, presence and machine queue folders only as it
    # first needs them, and spend-watch its own as it starts. Sessions.qml
    # lists the first two, and a folder that isn't there can't be listed,
    # so they're made at login, before the shell, with the other two
    # beside them so the shell's first look finds each. So are the
    # folders WaitingOnYou.qml watches for what waits on Joel.
    systemd.user.tmpfiles.rules = [
      "d %t/spend-watch - - - -"
      "d %t/jobs 0700 - - -"
      "d ${config.xdg.stateHome}/pi/agentic-harness.pi/mesh-workflow/presence - - - -"
      "d ${config.xdg.stateHome}/mission - - - -"
      "d ${config.xdg.stateHome}/mission/waiting - - - -"
      "d ${config.xdg.dataHome}/pi/agentic-harness.pi/quest-workflow/quests - - - -"
    ];

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

    # Where the laptop is, as GeoClue last found it, in the runtime
    # directory for the weather to watch (launcher/Weather.qml). A GeoClue
    # that refuses or goes is asked again a minute on, not at once, so a
    # machine without it isn't kept busy asking.
    systemd.user.services.whereabouts = lib.mkIf followsTheLaptop {
      Unit = {
        Description = "Says where the laptop is, for the launcher's weather";
        PartOf = [ "graphical-session.target" ];
        After = [ "graphical-session.target" ];
      };
      # A place Joel sets himself is pinned in its state folder, and Now's
      # Location sets and takes the pin away through the same program, on
      # the shell's path.
      Service = {
        StateDirectory = "whereabouts";
        ExecStart = "${lib.getExe whereabouts} %t/whereabouts.json %S/whereabouts/pin.json";
        Restart = "on-failure";
        RestartSec = 60;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };

    # The clock's zone follows where the laptop is (clock-follows/), asked
    # each time whereabouts writes it and never otherwise: a path unit
    # watches the file and starts the one run. What it did is kept in its
    # state folder for Now to say (Location.qml), and Put It Back starts
    # its other run, from the launcher.
    systemd.user.paths.clock-follows = lib.mkIf followsTheLaptop {
      Unit.Description = "Watches where the laptop is, for the clock's zone";
      Path.PathChanged = "%t/whereabouts.json";
      Install.WantedBy = [ "graphical-session.target" ];
    };
    systemd.user.services.clock-follows = lib.mkIf followsTheLaptop {
      Unit = {
        Description = "Moves the clock's zone to where the laptop is";
        # Each run reads only the latest fix, so a burst of them is never
        # too many. Five within ten seconds, systemd's limit, would fail
        # the run and its path unit with it, and the clock would follow
        # the laptop no more until the next login.
        StartLimitIntervalSec = 0;
      };
      Service = {
        Type = "oneshot";
        StateDirectory = "clock-follows";
        ExecStart = "${lib.getExe clockFollows} follow %t/whereabouts.json %S/clock-follows ${timedatectl}";
      };
    };
    systemd.user.services.clock-put-back = lib.mkIf followsTheLaptop {
      Unit.Description = "Puts the clock's zone back where it was before it followed the laptop";
      Service = {
        Type = "oneshot";
        StateDirectory = "clock-follows";
        ExecStart = "${lib.getExe clockFollows} put-back %S/clock-follows ${timedatectl}";
      };
    };
    # How the clock is kept, as timesyncd last said, in the runtime
    # directory for Time to watch: heard over D-Bus as each exchange with
    # the server is made, never asked on a clock nor through timedatectl.
    systemd.user.services.timekeeping = lib.mkIf keepsTime {
      Unit = {
        Description = "Says how the clock is kept, for the launcher's Time";
        PartOf = [ "graphical-session.target" ];
        After = [ "graphical-session.target" ];
      };
      Service = {
        ExecStart = "${lib.getExe timekeeping} %t/timekeeping.json";
        Restart = "on-failure";
        RestartSec = 60;
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
      # Bound to the mains supply, which systemd holds ready only while it's
      # online (nixos.nix), so it stops the moment the power goes, whatever
      # the shell heard; it never starts without it.
      BindsTo = [ "sys-subsystem-power_supply-mains.device" ];
      After = [
        "caffeinate.service"
        "sys-subsystem-power_supply-mains.device"
      ];
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
        # The bar, the launcher, notices and the lock screen are the
        # desktop's own, so they sit with the compositor in session.slice, at
        # its tenfold CPU weight and behind its memory floor (modules/slices),
        # rather than in app.slice beside Chrome and Slack, where a build's
        # pressure reaches them first.
        Slice = "session.slice";
        # Inside session.slice, first beside the compositor, ahead of the
        # portals and the bus at their tenth: the launcher is how Joel drives
        # the machine. Its floor is hard, so a build filling memory never
        # pushes it to swap; a gibibyte holds the 678 MB it used on 29
        # October, until the shell's own budget says less.
        CPUWeight = 1000;
        MemoryMin = "1G";
        Restart = "on-failure";
        RestartSec = 1;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };
  };
}
