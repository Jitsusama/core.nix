# Quickshell draws everything on Joel's screen that isn't a window: the
# wallpaper, the launcher on Super+Space, notifications, the lock screen,
# polkit's password prompt, SSH's askpass for a PIN or a yes, the volume,
# microphone, brightness and charge as they change, what's playing, and the
# margin's facts at the top of the screen.
# Each is a QML file beside this one, linked as it is; Theme.qml, written
# here from jitsusama.theme, gives them all the same look. It runs with the
# graphical session, and restarts if it ever falls over. What the margin
# and the launcher's foot show is
# jitsusama.quickshell.margin.pieces, rendered into Pieces.qml at build,
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
  # The key language's commands, by their whole names, for the : line.
  commands = lib.mapAttrs' (
    act: about:
    lib.nameValuePair about.command {
      inherit act;
      inherit (about) short;
      changes = about.changes or false;
      undoable = about.undoable or false;
      guarded = about.guarded or false;
      range = about.range or false;
      ends = about.ends or false;
      starts = about.starts or false;
    }
  ) (lib.filterAttrs (_: about: about ? command) config.jitsusama.keyLanguage.acts);
  # The marks the theme draws as paths, for Theme.drawn.
  drawn = lib.mapAttrs (_: mark: mark.drawn) (lib.filterAttrs (_: mark: mark.drawn != "") marks);
  # The marks the theme draws in pixels, for Theme.pixels.
  pixels = lib.mapAttrs (_: mark: mark.pixels) (lib.filterAttrs (_: mark: mark.pixels != [ ]) marks);
  slowed = milliseconds: builtins.floor (milliseconds * builtins.fromJSON motion.slowdown + 0.5);
  face = family: sizes: object ({ inherit family; } // lib.mapAttrs (_: builtins.fromJSON) sizes);

  # A level of the type hierarchy as a Text sets it, each value its font's
  # property of the same name, so a run of text names its level and nothing
  # else (type-is-set-by-level).
  setting =
    level:
    let
      size = step: builtins.fromJSON type.${level.face}.${step};
      # Qt spaces letters in pixels, which are a point's 4/3 at Linux's 96 to
      # the inch.
      sized = step: {
        pointSize = size step;
        letterSpacing = size step * 4 / 3 * level.tracking;
      };
      serif = level.face == "serif";
    in
    sized level.step
    // {
      family = font.${level.face}.family;
      # The serif is weighted by its axis; a weight of its own besides would
      # pick another instance under it, so it's the face's regular.
      weight = if serif then 400 else level.weight;
      variableAxes = level.axes;
      inherit (level) italic;
      # Font.MixedCase and Font.AllUppercase.
      capitalization = if level.case == "capitals" then 1 else 0;
      features = lib.genAttrs level.features (_: 1);
      colour = colours.${level.colour};
      # How far apart two lines of one thing at the level sit, baseline to
      # baseline (docs/design.md, Space).
      inherit (level) pitch;
    }
    // lib.optionalAttrs (level.wallpaper != null) { wallpaper = sized level.wallpaper; }
    # A line holding strong spans: its axes but the weight, which the face
    # gives at regular and each span at strong.
    // lib.optionalAttrs (level.strong != null) {
      spans = {
        variableAxes = removeAttrs level.axes [ "wght" ];
        weight = 400;
        inherit (level) strong;
      };
    };

  # Every file the shell links as it is, in a list for each place under
  # home/, named here so only a new place edits this one. A file two lists
  # give stops the evaluation, since it would be linked twice.
  qml =
    let
      joined =
        import ./home/accessories.nix
        ++ import ./home/agents.nix
        ++ import ./home/askpass.nix
        ++ import ./home/capture.nix
        ++ import ./home/clipboard.nix
        ++ import ./home/command-line.nix
        ++ import ./home/core.nix
        ++ import ./home/displays.nix
        ++ import ./home/emoji.nix
        ++ import ./home/heat.nix
        ++ import ./home/home.nix
        ++ import ./home/keep-awake.nix
        ++ import ./home/location.nix
        ++ import ./home/lock.nix
        ++ import ./home/looks.nix
        ++ import ./home/machine.nix
        ++ import ./home/margin.nix
        ++ import ./home/networks.nix
        ++ import ./home/notices.nix
        ++ import ./home/power.nix
        ++ import ./home/programs.nix
        ++ import ./home/selection.nix
        ++ import ./home/shell.nix
        ++ import ./home/shortcuts.nix
        ++ import ./home/sound.nix
        ++ import ./home/spend.nix
        ++ import ./home/timers.nix
        ++ import ./home/warp.nix
        ++ import ./home/windows.nix
        ++ import ./home/words.nix;
      twice = lib.filter (file: lib.count (other: other == file) joined > 1) (lib.unique joined);
    in
    if twice == [ ] then
      joined
    else
      throw "More than one of quickshell's places links ${lib.concatStringsSep ", " twice}";

  # A service holding one of logind's locks for as long as what it runs
  # does, nothing unless it says, with a file saying so.
  holding =
    {
      what,
      flag,
      runs ? [
        (lib.getExe' pkgs.coreutils "sleep")
        "infinity"
      ],
      ...
    }@unit:
    {
      Unit = removeAttrs unit [
        "what"
        "flag"
        "runs"
      ];
      Service = {
        ExecStart = lib.escapeShellArgs (
          [
            (lib.getExe' pkgs.systemd "systemd-inhibit")
            "--what=${what}"
            "--who=Keep Awake"
            "--why=Kept awake from the launcher"
            "--mode=block"
          ]
          ++ runs
        );
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
  # Each key's face by the key map's spelling of it, its xkb name in lower
  # case, and the three it spells otherwise: the one table a chord's keys
  # and a keyboard's most pressed are drawn from (Shortcuts.qml's faces).
  spelledFaces = lib.mapAttrs' (name: lib.nameValuePair (lib.toLower name)) keyFaces // {
    enter = keyFaces.Return;
    pageup = keyFaces.Page_Up;
    pagedown = keyFaces.Page_Down;
  };

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
      # A + alone is the key too, as Calendar's Add a calendar is: split,
      # it was an empty key, and its chord drew nothing.
      lonePlus = keys == "+" || lib.hasSuffix "++" keys;
      key = if lonePlus then "+" else lib.last (lib.splitString "+" keys);
      held = lib.init (lib.splitString "+" (lib.removeSuffix "+" keys));
      pressed = if lib.elem key wholeKeys then [ key ] else held ++ [ key ];
      face =
        word:
        {
          wide = false;
        }
        // (spelledFaces.${word} or {
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
      "reader"
      "zathura"
      "mpv"
      "chrome"
      "google-mail"
      "google-calendar"
      "google-meet"
      "google-docs"
      "google-sheets"
      "google-slides"
      "google-drive"
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
    # (launcher/words/).
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
        # Super+N opens For You's Desktop (launcher/Now.qml's leadsTo), on
        # the card in front while one is up, so the launcher steps into it.
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
    reader = nerd "f0219";
    # The theme's book, as the viewer shows documents of pages.
    zathura = nerd "f05da";
    # The theme's film, as the player plays films.
    mpv = nerd "f0381";
    # The theme's web, as the browser goes about it.
    chrome = nerd "f059f";
    # Google's programs wear what they wear in Programs
    # (launcher/Provider.qml's programGlyphs): one thing, one mark.
    google-mail = nerd "f02ab";
    google-calendar = mark "calendar";
    google-meet = nerd "f0567";
    google-docs = nerd "f0dc8";
    google-sheets = nerd "f09f7";
    google-slides = nerd "f0428";
    google-drive = nerd "f02b6";
    # The theme's chat, as Slack is where people talk.
    slack = nerd "f0b79";
    # The theme's password, as 1Password keeps them.
    onepassword = nerd "f0306";
    # The theme's brush, as GIMP paints on pictures.
    gimp = nerd "f00e3";
    # The theme's vector, as Inkscape draws in shapes and paths.
    inkscape = mark "vector";
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
    homeNetwork = ./pieces/HomeNetwork.qml;
    workNetwork = ./pieces/WorkNetwork.qml;
    volume = ./pieces/Volume.qml;
    awake = ./pieces/Awake.qml;
    profile = ./pieces/Profile.qml;
    disk = ./pieces/Disk.qml;
    lag = ./pieces/Lag.qml;
    spend = ./pieces/Spend.qml;
    tokens = ./pieces/Tokens.qml;
    notices = ./pieces/Waiting.qml;
    forYou = ./pieces/ForYou.qml;
    quiet = ./pieces/DoNotDisturb.qml;
    microphone = ./pieces/Microphone.qml;
    capslock = ./pieces/CapsLock.qml;
    layer = ./pieces/Layer.qml;
    sharing = ./pieces/Sharing.qml;
    camera = ./pieces/Camera.qml;
    pi = ./pieces/Pi.qml;
    recording = ./pieces/Recording.qml;
    playing = ./pieces/Playing.qml;
    timers = ./pieces/Timers.qml;
  };

  placed = pieces.drawn config.jitsusama.quickshell.margin.pieces;

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
    timers = ./launcher/Timers.qml;
    weather = ./launcher/Weather.qml;
    ask = ./launcher/Ask.qml;
    machine = ./launcher/Machine.qml;
    power = ./launcher/Power.qml;
    heat = ./launcher/Heat.qml;
    traffic = ./launcher/Traffic.qml;
    waits = ./launcher/Waits.qml;
    firmware = ./launcher/Firmware.qml;
    warp = ./launcher/Warp.qml;
    machines = ./launcher/Machines.qml;
    today = ./launcher/Today.qml;
    light = ./launcher/Light.qml;
    keyboards = ./launcher/Keyboards.qml;
    emoji = ./launcher/Emoji.qml;
    shortcuts = ./launcher/KeyMap.qml;
    selection = ./launcher/OnSelection.qml;
    keyboard = ./launcher/Keyboard.qml;
    clipboard = ./launcher/Copies.qml;
    notices = ./launcher/PastNotices.qml;
    on = ./launcher/Now.qml;
    wifi = ./launcher/Wireless.qml;
    eero = ./launcher/HomeNetwork.qml;
    sound = ./launcher/Sound.qml;
    media = ./launcher/Players.qml;
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

  # The program that hears every copy, for the shell's acts on a copy and
  # the clipboard collection.
  clipboardHistory = pkgs.callPackage ./clipboard/package.nix { };

  # The package the system installs warp-cli by, WARP's own client, found by
  # the program it says it is, or null where the system installs none.
  systemWarpCli = lib.findFirst (
    package: lib.isDerivation package && (package.meta.mainProgram or "") == "warp-cli"
  ) null (osConfig.environment.systemPackages or [ ]);
  keepsCopies = launcher.providers.clipboard.enable or false;
  # Emoji's rows are served by the clipboard's history, which pastes them,
  # from the list in the tone the collection names.
  listsEmoji = launcher.providers.emoji.enable or false;
  emojiListed = emojiList (launcher.providers.emoji.tone or "none");

  # The daemon that says where the laptop is, for the weather's here, and
  # the zone the clock keeps. It asks GeoClue only while the weather may
  # ask the network, since it has GeoClue ask BeaconDB, and where the
  # machine runs GeoClue (nixos.nix).
  whereabouts = pkgs.callPackage ./whereabouts/package.nix { };
  clockFollows = pkgs.callPackage ./clock-follows/package.nix { };
  timedatectl = lib.getExe' pkgs.systemd "timedatectl";
  # The program that says how the clock is kept, for Time's insides, where
  # the machine keeps it with chrony.
  timekeeping = pkgs.callPackage ./timekeeping/package.nix { };
  keepsTime = osConfig.services.chrony.enable or false;
  # The program that keeps Joel's timers and rings them, for Timers.
  timers = pkgs.callPackage ./timers/package.nix { };
  keepsTimers = launcher.providers.timers.enable or false;
  # Where the system installs warp-cli, warp holds WARP's link for Network.
  keepsWarp = launcher.providers.warp.enable or false;
  # What a timer coming due is heard with: pi's loud, low, dog-safe ding
  # where the machine has pi (modules/pi/chime.nix), played by pw-play,
  # quietly while a-call-is-on says a call is on. A machine without pi
  # rings its timers as notices alone.
  timerChime = config.jitsusama.pi.chime or null;
  timerSound = lib.optionals (timerChime != null) [
    timerChime.player
    timerChime.ding
    (lib.getExe (pkgs.callPackage ../calls/package.nix { }))
  ];
  # What a timer's notice says, from the launcher's vocabulary, so a word
  # is said in one place.
  timerWords =
    let
      named = id: (lib.findFirst (thing: thing.id == id) { } launcher.words.things).name or id;
      said = (lib.findFirst (thing: thing.id == "timer") { } launcher.words.things).said or { };
    in
    pkgs.writeText "timer-words.json" (
      builtins.toJSON {
        app = named "timers";
        unnamed = named "timer";
        rang = said.notice;
        late = said.noticeLate;
        again = named "verb:timer:again";
        inherit (said) hours minutes seconds;
      }
    );
  # Whether shutting the lid suspends the machine on battery or on the
  # charger, as logind does unless told otherwise; the lid module has it
  # suspend on battery. Only where it suspends does keeping awake need to
  # hold the lid too. A setting left unsaid counts as suspending, so a
  # machine that says only what its lid does on battery, as the test VM's
  # does, still holds it, which changes nothing there.
  lidSleeps =
    let
      login = osConfig.services.logind.settings.Login or { };
    in
    (login.HandleLidSwitch or "suspend") != "ignore"
    || (login.HandleLidSwitchExternalPower or "suspend") != "ignore";
  followsTheLaptop =
    (launcher.providers.weather.enable or false)
    && (launcher.providers.weather.lookups or launcher.lookups)
    && (osConfig.services.geoclue2.enable or false);

  # Every emoji, in a skin tone.
  emojiList = import ./launcher/emoji.nix { inherit lib pkgs; };
  timeZones = import ./launcher/zones.nix { inherit lib pkgs; };
  characterNames = import ./launcher/names.nix { inherit lib pkgs; };

  # The launcher's vocabulary in a file of its own in the store, for what
  # can read no home: the terminal's launcher, and wifid, a system service
  # handed it by its path as the launcher watches its rows, which words
  # them in it.
  launcherWords = pkgs.writeText "words.json" (builtins.toJSON launcher.words);

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

  # Named as the margin's pieces are, so two never share a file.
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
  # listening, as while it restarts, qs ipc carries it instead. The line
  # ends with when the chord ran it, so the shell can time an opening from
  # Joel's press rather than from when it got round to hearing it. It's one
  # static program whose pages launcher-say-resident holds in memory, so a
  # chord reads nothing from the disk (launcher-say/src/main.rs says why).
  launcherSay = pkgs.callPackage ./launcher-say/package.nix { };

  # What the shell weighs, part by part, read from outside it so its own
  # thread pays nothing (shell-weighs/src/main.rs says how).
  shellWeighs = pkgs.callPackage ./shell-weighs/package.nix { };

  # How `launcher` (in-a-terminal/) inks a terminal: each role it draws in
  # as a 24-bit escape, and as the terminal colour holding the role's
  # pigment or near it (../theme/nearest.nix), or the terminal's own
  # foreground where none is; each mark with its console stand-in; and the
  # mark a failure wears.
  terminalLook =
    let
      inherit (config.jitsusama.theme) terminal;
      eight = [
        "black"
        "red"
        "green"
        "yellow"
        "blue"
        "magenta"
        "cyan"
        "white"
      ];
      sixteen = eight ++ map (name: "bright_${name}") eight;
      slot =
        role:
        let
          at = import ../theme/nearest.nix { inherit lib; } colours.${role} (
            map (name: terminal.${name}) (sixteen ++ [ "foreground" ])
          );
        in
        if at == null || at == 16 then "39" else toString (if at < 8 then 30 + at else 82 + at);
      roles = [
        "text"
        "strong"
        "muted"
        "alert"
        "warning"
        "success"
        "action"
        "program"
        "machine"
        "person"
        "voice"
      ];
    in
    pkgs.writeText "launcher-look.json" (
      builtins.toJSON {
        roles = lib.genAttrs roles (role: {
          truecolour = import ../theme/sgr.nix { inherit lib; } "38" colours.${role};
          sixteen = slot role;
        });
        marks = lib.mapAttrsToList (_: mark: { inherit (mark) glyph console; }) marks;
        failed = {
          inherit (marks.failed) glyph console;
        };
      }
    );

  # `launcher`, the launcher in a terminal (in-a-terminal/): what it asks
  # the shell through, how it inks and the words it says, from the
  # launcher's own vocabulary; and zsh's completion, as it writes it.
  launcherInATerminal =
    let
      program = lib.getExe (pkgs.callPackage ./in-a-terminal/package.nix { });
    in
    pkgs.runCommand "launcher"
      {
        nativeBuildInputs = [ pkgs.makeWrapper ];
        meta.mainProgram = "launcher";
      }
      ''
        makeWrapper ${program} $out/bin/launcher \
          --set LAUNCHER_QS ${lib.getExe' pkgs.quickshell "qs"} \
          --set LAUNCHER_LOOK ${terminalLook} \
          --set LAUNCHER_WORDS ${launcherWords}
        mkdir -p $out/share/zsh/site-functions
        $out/bin/launcher completion zsh > $out/share/zsh/site-functions/_launcher
        grep -q '^#compdef launcher$' $out/share/zsh/site-functions/_launcher
      '';

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

  # Locks the screen as the lid shuts while Keep Awake holds it, away from a
  # desk (./lid-held.sh).
  lidHeld = pkgs.writeShellApplication {
    name = "lid-held";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.glib
      pkgs.gnugrep
      pkgs.quickshell
      pkgs.systemd
    ];
    text = builtins.readFile ./lid-held.sh;
  };

  # Every file the shell reads, by its store path, and the list of them its
  # start links, so a switch that changes any is told from one that doesn't.
  shellFiles = lib.mapAttrsToList (_: file: "${file.source}") (
    lib.filterAttrs (name: _: lib.hasPrefix "quickshell/" name) config.xdg.configFile
  );
  shellFilesList = pkgs.writeText "quickshell-files" (lib.concatLines shellFiles);

  # Compiles the shell's files before the shell restarts onto them (./starts-warm.sh).
  startsWarm = pkgs.writeShellApplication {
    name = "quickshell-starts-warm";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.gnused
      pkgs.quickshell
      pkgs.systemd
    ];
    text = builtins.readFile ./starts-warm.sh;
  };
in
{
  imports = [
    ../theme/home.nix
    ../keys/home.nix
    ./recents.nix
    # So a program's module that brings no shell can still add a provider.
    ./providers.nix
    ./command-lines.nix
    ../camera/home.nix
    # The brightness bar shows the picks the backlight daemon tells it of.
    ../backlight/home.nix
    # The pieces were named for niri's overview, where they first stood;
    # a machine that still sets the old name reaches the new one, warned.
    (lib.mkRenamedOptionModule
      [ "jitsusama" "quickshell" "overview" "pieces" ]
      [ "jitsusama" "quickshell" "margin" "pieces" ]
    )
    # The option's first name said quiet, a word the glossary replaces with
    # Do not disturb; a machine setting it still reaches the new one.
    (lib.mkRenamedOptionModule
      [ "jitsusama" "quickshell" "quiet" "whileSharing" ]
      [ "jitsusama" "quickshell" "doNotDisturb" "whileSharing" ]
    )
    # Machine's answers are what the machine daemon holds.
    ../machine/home.nix
    # Programs, each program's drawer and its Recent are what the
    # programs daemon holds.
    ../programs/home.nix
    # Sound's outputs and inputs are what the sound daemon holds.
    ../sound/home.nix
    # Windows, Capture and each program's windows are what the windows
    # daemon holds.
    ../windows/home.nix
  ];

  options.jitsusama.quickshell.margin.pieces = lib.mkOption {
    type = lib.types.attrsOf (
      pieces.piece {
        inherit (config.jitsusama.theme) marks;
        sources = builtIn;
      }
    );
    description = ''
      What the margin and the launcher's foot show, by name. Each piece has its order, tier,
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
          # The furthest of all, so a sender named only for its other
          # settings, as Slack's module names its daemon, reaches as far as
          # it asks, as one not named does.
          default = "urgent";
          description = ''
            The furthest a sender's notices reach: kept in the history alone,
            counted as waiting in the margin but never shown, shown as a card,
            or, when the notice is critical, shown as one that stays until
            it's answered. A notice reaches less far when it asks for less,
            never further than this.
          '';
        };
        options.addressed = lib.mkOption {
          type = lib.types.nullOr (
            lib.types.submodule {
              options.names = lib.mkOption {
                type = lib.types.listOf lib.types.str;
                default = [ ];
                description = "The names a mention of Joel reads as, without the @.";
              };
              options.bots = lib.mkOption {
                type = lib.types.listOf lib.types.str;
                default = [ ];
                description = "Senders whose direct messages are a program's, not a person's.";
              };
              options.everyMessage = lib.mkOption {
                type = lib.types.listOf lib.types.str;
                default = [ ];
                description = "The channels Joel set to tell him every message, without the #.";
              };
            }
          );
          default = null;
          description = ''
            Who Joel is in a chat program's notices, said as Slack's are, so
            only what's to him sounds (`sounds.addressed`): a person's direct
            message, a mention, or a channel's notice from one not set to
            tell him every message. The rest is counted, without a card. A
            program without this is judged by its level alone.
          '';
        };
        options.posted = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = ''
            The socket of the program's daemon that says whether a notice
            from a channel Joel reads every message of was a post or a
            reply in a thread he follows, under the runtime folder where it
            isn't absolute. Its module gives it; with `addressed`, a reply
            is to him and sounds.
          '';
        };
      }
    );
    default = { };
    example = {
      Spotify.level = "history";
      Slack = {
        level = "card";
        addressed = {
          names = [ "ada" ];
          everyMessage = [ "team" ];
        };
      };
    };
    description = ''
      How far each program's notices reach, by the name a notice gives as
      its sender, in any case. A program not named reaches as far as it
      asks.
    '';
  };

  options.jitsusama.quickshell.sounds.addressed = lib.mkOption {
    type = lib.types.nullOr lib.types.path;
    default = pkgs.runCommand "notices-tock.wav" { nativeBuildInputs = [ pkgs.sox ]; } ''
      ${lib.getExe pkgs.bash} ${../notices/tock.sh} $out
    '';
    defaultText = lib.literalMD "a soft, low knock on wood, struck by modules/notices/tock.sh";
    description = ''
      The sound a message to Joel himself makes as its card shows: one the
      notices daemon says chimes, from a chat program the machine says who
      he is in (`notices.senders.<name>.addressed`). Quieter and shorter
      than the want bell, so it's never taken for it; never while notices
      are held, nor while a call is on. Null makes none.
    '';
  };

  options.jitsusama.quickshell.sounds.wants = lib.mkOption {
    type = lib.types.nullOr lib.types.path;
    default =
      config.jitsusama.pi.chime.want
        or "${pkgs.sound-theme-freedesktop}/share/sounds/freedesktop/stereo/message.oga";
    defaultText = lib.literalExpression ''config.jitsusama.pi.chime.want, or "''${pkgs.sound-theme-freedesktop}/share/sounds/freedesktop/stereo/message.oga" without pi'';
    description = ''
      The bell the shell rings, at half volume, when a pi session
      first stands stopped on Joel's keys, a question or a yes, so he hears
      it from across the room, with a notice that takes him to it; never
      for a blocked run or an item, while notices are held, nor for a want
      he's already looking at. Where the machine has pi it's pi's low bell
      struck twice (modules/pi/chime.nix), since the theme's message sits
      at 840 Hz to 5 kHz, where it scared Joel's dog. Null makes none.
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

  options.jitsusama.quickshell.doNotDisturb.whileSharing = lib.mkOption {
    type = lib.types.bool;
    default = true;
    description = ''
      Whether do not disturb is on by itself while another program sees the
      screen, as a call shares it, and off again once nothing does. Ending it
      meanwhile lets notices show until the sharing stops.
    '';
  };

  # The launcher's caret blinks as kitty's cursor does (modules/kitty/
  # kitty.conf), so Neovim's and the launcher's read alike.
  options.jitsusama.quickshell.caret = {
    fade = lib.mkOption {
      type = lib.types.ints.unsigned;
      default = 800;
      description = ''
        How long the launcher's caret takes to fade out, and again to come
        back, in milliseconds, as kitty's cursor_blink_interval; 0 holds it
        steady.
      '';
    };
    stopsAfter = lib.mkOption {
      type = lib.types.ints.unsigned;
      default = 15000;
      description = ''
        How long after the last key the launcher's caret stops blinking and
        holds steady, in milliseconds, as kitty's cursor_stop_blinking_after,
        so a launcher left still draws nothing more.
      '';
    };
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
      default = (import ./launcher/words.nix).value;
      defaultText = lib.literalExpression "(import ./launcher/words.nix).value";
      description = ''
        The launcher's vocabulary, as launcher/words.nix joins it from each
        place's file: its
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
        # The home network's own rows (docs/glossary.md, Network).
        internet = "internet";
        node = "node";
        "eero device" = "device";
        profile = "profile";
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
    # What was taken lately (Taken.qml): the folders screenshots and
    # recordings land in, which the programs daemon reads itself as Taken
    # watches them, under listed:shared:taken, so the shell runs no find.
    jitsusama.quickshell.launcher.recents = [
      {
        name = "taken";
        folders = [
          "${config.home.homeDirectory}/Pictures/Screenshots"
          "${config.home.homeDirectory}/Videos/Recordings"
        ];
      }
    ];

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
        # A player's art, in its Media's drawer, may be the network's.
        inherit (launcher) lookups;
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
      # Joel's timers, beside the time, which the timers daemon keeps and
      # rings.
      timers = {
        order = 95;
        source = "timers";
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
      # Whether anything Joel uses waited, from lag-watch, only where it
      # runs, so a machine without it names no Lag in Machine and one with it
      # always does, before its first ten seconds too.
      waits = {
        order = 93;
        source = "waits";
        enable = config.systemd.user.services ? lag-watch;
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
      # How WARP's link stands, from warp (modules/warp) asking the warp-cli
      # the system installs, only where it installs one, so a machine without
      # WARP has no WARP in Network.
      warp = {
        order = 89;
        source = "warp";
        enable = systemWarpCli != null;
        warp = lib.getExe (pkgs.callPackage ../warp/package.nix { });
        warpCli = if systemWarpCli != null then lib.getExe systemWarpCli else "";
      };
      # Every machine of the realms this one's mycelium nodes run, each
      # node's records watched; a machine with no node lists itself alone.
      machines = {
        order = 92;
        source = "machines";
        weight = 0.9;
        realms =
          lib.optional ((config.jitsusama.mycelium.realm or null) != null) config.jitsusama.mycelium.realm
          ++ lib.attrNames (config.jitsusama.mycelium.alsoJoins or { });
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
      # The home network the eero daemon keeps (modules/eero): the
      # internet, its nodes and the devices on them, only where the
      # daemon runs, so a machine without it names no Home Network in
      # Network. It holds nothing until the daemon has told its record.
      eero = {
        order = 102;
        source = "eero";
        weight = 0.9;
        enable = config.systemd.user.services ? eero-daemon;
      };
      # The devices sound goes out of and comes in through.
      sound = {
        order = 103;
        source = "sound";
        weight = 0.9;
      };
      # Every media player and what it plays, beside Sound, which holds the
      # devices it plays through.
      media = {
        order = 103;
        source = "media";
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
      # What Agents knows of the orchestrators: who they are, the talks
      # Joel has had with them and how a closed one reopens, no place of
      # its own, since each is a row of Agents' tree.
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
        # What's playing's art, in its drawer, may be the network's.
        inherit (launcher) lookups;
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
      # What an agent can do with what's selected, opened by Super+X alone,
      # where the account has pi, whose pi-verbs serves the actions and
      # whose pi-ask answers them.
      selection = {
        order = 125;
        source = "selection";
        enable = lib.hasAttrByPath [ "pi" "ask" ] config.jitsusama;
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
    jitsusama.quickshell.margin.pieces = lib.mapAttrs (_: lib.mapAttrs (_: lib.mkDefault)) {
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
      # Only while something ails the home network, from the eero daemon,
      # beside the network this machine is on.
      homeNetwork = {
        order = 41;
        source = "homeNetwork";
        label = "home network";
        tier = "when";
        mark = "accessPoint";
      };
      # Only while something ails the link to work, from the warp daemon,
      # beside the home network.
      workNetwork = {
        order = 42;
        source = "workNetwork";
        label = "work network";
        tier = "when";
        mark = "tunnel";
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
      # The AI tokens running out at this rate, or out until they reset,
      # where spend-watch runs and only while it warns: said as a notice
      # and listed in Now, and a glyph in the margin, beside the day's
      # spend.
      tokens = {
        order = 51;
        source = "tokens";
        label = "tokens";
        tier = "when";
        mark = "tokens";
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
      };
      # What's on only now and then, each also a glyph in the margin while
      # it's true (Margin.qml), in the order the pieces give them.
      # The notices missed wear no mark of their own: what of them waits on
      # Joel is For You's, whose tray says it (Joel's pick, 2026-11-09).
      notices = {
        order = 54;
        source = "notices";
        label = "notices";
        tier = "when";
      };
      # The one mark for what waits on Joel, while For You holds any: a
      # session wanting him, a to-do, a look, a landing to try or a
      # critical notice (pieces/ForYou.qml).
      forYou = {
        order = 55;
        source = "forYou";
        label = "for you";
        tier = "when";
        mark = "forYou";
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
      # A timer running, paused or rung, or the stopwatch running, wearing
      # the mark Timers' place wears (VKCF3Z, 2026-11-08).
      timers = {
        order = 92;
        source = "timers";
        label = "timers";
        tier = "when";
        mark = "timer";
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
    # copies it, and notify-send says where the file went. A terminal
    # shows a run's answer through jq (launcher/Runs.qml). A paste with no
    # clipboard history kept presses its window's paste keys through wtype
    # (Pasting.qml).
    # Where the clock follows the laptop, clock-follows too, which typed
    # bare says where it last moved the clock from the service's state.
    home.packages =
      lib.optionals followsTheLaptop [
        whereabouts
        clockFollows
      ]
      ++ [
        askpass
        launcherInATerminal
        launcherSay
        pkgs.quickshell
        # What the timer writes to the journal, at hand: typed in a terminal,
        # it says what the shell weighs now, part by part.
        shellWeighs
        pkgs.wf-recorder
        (pkgs.callPackage ./record-window/package.nix { })
        pkgs.jq
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
        ) placed
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
        # Which places the launcher has on, by the names they're turned on
        # under, for cadence to read its journal by (80ECO2, 2026-11-11):
        # written as the shell's places are, from the same list, so it can't
        # say other than what the shell loads, and nothing runs to write it.
        "quickshell/launcher/places-on.json".text = builtins.toJSON {
          on = lib.sort lib.lessThan (map (provider: provider.name) providers);
        };
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
                    # The zones, and which tzdata they came from, which
                    # Time's Zone says, and which chrony keeps the clock,
                    # which its Clock says, where chrony does.
                    // lib.optionalAttrs (provider.source == "time") {
                      list = "${timeZones}";
                      tzdata = pkgs.tzdata.version;
                    }
                    // lib.optionalAttrs (provider.source == "time" && keepsTime) {
                      chrony = osConfig.services.chrony.package.version;
                    }
                    // lib.optionalAttrs (provider.source == "characters") {
                      names = "${characterNames}";
                    }
                    # The words and number settings wifid says Wi-Fi's
                    # rows in, and the eero daemon the Home Network's,
                    # each a file in the store, since neither reads
                    # anything else.
                    //
                      lib.optionalAttrs
                        (builtins.elem provider.source [
                          "wifi"
                          "eero"
                        ])
                        {
                          words = "${launcherWords}";
                          numbers = "${pkgs.writeText "numbers.json" (
                            builtins.toJSON {
                              group = provider.group or "";
                              point = provider.point or "";
                            }
                          )}";
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
              readonly property var doNotDisturb: ${object config.jitsusama.quickshell.doNotDisturb}
              readonly property bool reports: ${lib.boolToString config.jitsusama.quickshell.reports}
              // How the launcher's caret blinks, in milliseconds.
              readonly property var caret: ${object config.jitsusama.quickshell.caret}
              // How long keys wait, in milliseconds, from the key language
              // (modules/keys/language.toml), as Neovim reads it too.
              readonly property var keyTiming: ${object config.jitsusama.keyLanguage.timing}
              // Every command the : line can name, by its whole name, with
              // its act, fixed short form and flags, from the key language.
              readonly property var commands: ${object commands}
              // Each viewer's command line, which its : opens, from its
              // module (command-lines.nix).
              readonly property var commandLines: ${object launcher.commandLines}
              // Whether a shut lid suspends the machine, on battery or on the
              // charger.
              readonly property bool lidSleeps: ${lib.boolToString lidSleeps}
              readonly property var margin: ${object config.jitsusama.quickshell.margin}
              // The sound a want makes and a message to Joel makes, what plays
              // them, and what says a call is on; "" for none.
              readonly property var sounds: ${
                object {
                  wants = toString config.jitsusama.quickshell.sounds.wants;
                  addressed = toString config.jitsusama.quickshell.sounds.addressed;
                  player = lib.getExe' pkgs.pipewire "pw-play";
                  calls = lib.getExe (pkgs.callPackage ../calls/package.nix { });
                }
              }
              // Each sender's level the notices daemon keeps it to, from its
              // own file (modules/notices), not from here.
              readonly property var notices: ${
                object {
                  inherit (config.jitsusama.quickshell.notices) away;
                  # The senders the daemon judges by who Joel is in them, in
                  # lower case, whose pieces are muted until one is to him.
                  addressed =
                    let
                      inherit (config.jitsusama.quickshell.notices) senders;
                      named = lib.filterAttrs (_: sender: sender.addressed != null) senders;
                    in
                    map lib.toLower (lib.attrNames named);
                }
              }
              // Other modules' surfaces, by file, from jitsusama.quickshell.surfaces.
              readonly property var surfaces: ${
                builtins.toJSON (map (name: "${name}.qml") (lib.attrNames config.jitsusama.quickshell.surfaces))
              }
          }
        '';
        "quickshell/Pieces.qml".text = ''
          pragma Singleton
          import Quickshell
          import QtQuick

          // The margin's pieces, in order, from jitsusama.quickshell.margin.pieces.
          Singleton {
              readonly property var inOrder: ${
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
                  ) placed
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
              // Each key's face, { text, wide }, by the key map's lower-case
              // xkb name for it, as every chord here is drawn.
              readonly property var faces: ${builtins.toJSON spelledFaces}
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
              // Light text on dark or dark on light, as Vim's background
              // says, set when the system is built.
              readonly property string appearance: "${config.jitsusama.theme.appearance}"
              readonly property var mono: ${face font.mono.family type.mono}
              // The voice's face, set by its variable axes (font.variableAxes)
              // with its weight among them, each step sized by capitals. There's
              // no sans: what the shell sets for reading is in the serif upright
              // at large, as the reader's page is (Joel, 4 November: "generally,
              // not a fan of sans-serif").
              readonly property var serif: ${
                object (
                  {
                    inherit (font.serif)
                      family
                      italic
                      weight
                      tracking
                      ;
                    axes = font.serif.axes // {
                      wght = font.serif.weight;
                    };
                    # Its axes set upright, or italic but larger, at a step,
                    # whole: the voice's laid under each.
                    upright = lib.mapAttrs (_: axes: font.serif.axes // axes) font.serif.upright;
                    # The same but for the weight, for a line whose spans set
                    # their own: Qt's rich text can't set an axis in a span,
                    # and a weight axis on the line outweighs every span's, so
                    # such a line takes its weight from the face's own.
                    unweighted = lib.mapAttrs (
                      _: axes: removeAttrs (font.serif.axes // axes) [ "wght" ]
                    ) font.serif.upright;
                    slanted = lib.mapAttrs (_: axes: font.serif.axes // axes) font.serif.slanted;
                  }
                  // lib.mapAttrs (_: builtins.fromJSON) type.serif
                )
              }
              // The script face's cut for words known to be in one script, by
              // the face beside it and the script, as script.serif.hebrew: Qt
              // falls back by script, not language, so Chinese is drawn in its
              // own forms only where its cut is named.
              readonly property var script: ${object { inherit (font.script) serif mono; }}
              // The line above a name, saying when, where or who.
              readonly property var eyebrow: ${object type.eyebrow}
              // The type hierarchy's levels, by name, as Theme.level.name.pointSize:
              // every run of text names one (docs/design.md, Type).
              readonly property var level: ${object (lib.mapAttrs (_: setting) type.levels)}
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
              // A find's ink and ground, as Joel's Neovim draws a search's
              // (modules/neovim/colors/jitsusama.lua, Search): the notices' /
              // washes what it finds with it, and the launcher the letters
              // its find line matched. On the selection, which lies a shade
              // past raised, its ground is the shade past the selection,
              // rule's, so a selected row's find still shows. No find takes
              // the accent: the find line has no match the cursor is on.
              readonly property var wash: ${
                object {
                  ink = colours.strong;
                  ground = colours.raised;
                  selected = colours.rule;
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
                  # niri's spring, as its workspace slide rides it, for what
                  # moves with that slide, as the margin's cat does.
                  spring = {
                    damping = builtins.fromJSON motion.spring.damping;
                    inherit (motion.spring) stiffness;
                    epsilon = builtins.fromJSON motion.spring.epsilon;
                  };
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
              // The marks drawn in pixels, the empty places' cat: each row a
              // string, # for ink (PixelMark.qml).
              readonly property var pixels: ${object pixels}
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

    # pi makes its quests, presence and machine queue folders only as it
    # first needs them, and spend-watch its own as it starts. Sessions.qml
    # lists the first two, and a folder that isn't there can't be listed,
    # so they're made at login, before the shell, with the other two
    # beside them so the shell's first look finds each. So are the
    # folders of what waits on Joel, of the harness's index of quests, of
    # its runs, and of its steer inboxes, whose talks the mycelium node
    # serves, as it does the rest, to WaitingOnYou.qml and launcher/Runs.qml
    # and Orchestrators.qml: the node watches a folder made after it starts
    # only once a minute has passed.
    systemd.user.tmpfiles.rules = [
      "d %t/spend-watch - - - -"
      "d %t/jobs 0700 - - -"
      "d ${config.xdg.stateHome}/pi/agentic-harness.pi/mesh-workflow/presence - - - -"
      "d ${config.xdg.stateHome}/pi/agentic-harness.pi/mesh-workflow/runs - - - -"
      "d ${config.xdg.stateHome}/pi/agentic-harness.pi/steer - - - -"
      "d ${config.xdg.stateHome}/mission - - - -"
      "d ${config.xdg.stateHome}/mission/waiting - - - -"
      "d ${config.xdg.stateHome}/pi/agentic-harness.pi/quest-workflow - - - -"
      "d ${config.xdg.dataHome}/pi/agentic-harness.pi/quest-workflow/quests - - - -"
    ];

    # The kitty every session's window opens in (Sessions.qml's
    # inSessions), kept running with a hidden window of its own. kitty's
    # single instance quits as its last window closes, and a session asked
    # of it as it quit was lost without a word: Reopen Them All asked for
    # one 0.07 s before the last session's kitty quit, and it never opened
    # (2026-11-04), so its hidden window keeps it running. Windows are
    # asked of it over remote control, on a socket in a runtime folder only
    # Joel's login can open, rather than through kitty's single instance:
    # that instance answers on a name any program can take first, a fenced
    # command among them, which would then have every session's window
    # opened in it (ISXEG8, 2026-11-11). The fence empties the runtime
    # folder, so no fenced command reaches the socket, and kitty takes
    # remote control only there, never through a window's terminal. Any
    # unfenced program of Joel's can use the socket, to read or type into
    # every session window; that same account can already read each
    # session's environment, so the fence stays the boundary. A password
    # wouldn't narrow it: kitty asks on screen about a request with an
    # unknown one, which a program could repeat into prompts for Joel
    # (ISXEG8, 2026-11-12). It runs in
    # agents.slice, where kitty puts each session's scope. Its window is
    # classed pi-sessions-kept, so the shell, which numbers that kitty's
    # windows as niri lists them, leaves out the one niri never shows.
    systemd.user.services.sessions-kitty = {
      Unit = {
        Description = "The kitty every session's window opens in, kept between sessions";
        # Every session's window is one of its windows, so a switch that
        # restarted it closed them all: at 06:46 on 2026-11-09 kitty 0.50
        # changed the unit and every session's window went, the sessions
        # left running without one. A changed kitty is taken at the next
        # login instead, as the SSH agent's is.
        X-SwitchMethod = "keep-old";
        PartOf = [ "graphical-session.target" ];
        After = [ "graphical-session.target" ];
        # Kept trying while the session's display isn't there yet.
        StartLimitIntervalSec = 0;
      };
      Service = {
        Slice = "agents.slice";
        # This one process draws every session's window and reads every key
        # Joel types into one, so it comes before the sessions it holds.
        # Beside its 71 sessions' scopes at the default weight it had a
        # seventy-second of the slice, and with no floor of its own 628 MB
        # of it sat in swap against 88 MB resident while agents.slice held
        # 25.8 GB over its 10 GB floor (2026-11-09). Opening a window then
        # paged its drawing back in under the disk's load: Joel's typing
        # stopped showing, every window redrew at once and keys went
        # missing. Its scopes take its slice, not its weight or its floor,
        # so the sessions themselves stay where they were. Its floor is
        # hard, since a soft one was taken back 285,154 times in 40 minutes
        # of swapping, and agents.slice keeps a hard floor the same size for
        # it (modules/slices). It covers what it held resident and hot, not
        # the scrollback swapped out cold.
        CPUWeight = 1000;
        MemoryMin = "512M";
        # What it swaps out stays compressed in memory rather than going to
        # disk, so a session's window Joel comes back to draws from memory.
        # At 20:38 on 2026-11-11 it held 1.18 GB in swap and 0.4 MB of that
        # in zswap, the rest on disk, with 306,506 pages faulted back from
        # swap since the morning. Scrollback is text cells and squeezes
        # well, and a pi window held near the focus keeps its own memory off
        # the disk the same way (jobs-sorter's src/focus.rs).
        MemoryZSwapWriteback = false;
        # It draws a frame only when niri asks for one, unlike Joel's own kitty
        # (kitty.conf's sync_to_monitor no). Drawing as it pleased, it drew
        # every session's window, hidden ones too, since niri never marks one
        # suspended: 106 ms of its thread a second at the median and 188 at
        # the 95th, 56% of it making each window's context current, and it
        # woke niri out of 505 of its 603 sleeps of 8 ms or more (7QEY1S,
        # 2026-11-10). niri asks a window it doesn't show for about one frame
        # a second, so the hidden ones go quiet; a shown one waits at most a
        # frame, 8.3 ms at 120 Hz.
        ExecStart = lib.concatStringsSep " " [
          "${config.home.profileDirectory}/bin/kitty"
          "--listen-on unix:%t/pi-sessions/kitty.sock -o allow_remote_control=socket-only"
          "-o sync_to_monitor=yes"
          "--class pi-sessions-kept --start-as=hidden ${lib.getExe' pkgs.coreutils "sleep"} infinity"
        ];
        # Its pid beside the socket, since a path names no pid the way
        # kitty's own single-instance name does, and the shell finds a
        # session's window among those niri lists by the pid drawing it
        # (Sessions.qml's kittyPid). It goes with the folder when the unit
        # stops; `$$` hands sh the variable systemd sets for the main pid.
        ExecStartPost = "${pkgs.runtimeShell} -c 'echo \"$$MAINPID\" > %t/pi-sessions/kitty.pid'";
        # The socket's folder, which only Joel's login may open.
        RuntimeDirectory = "pi-sessions";
        RuntimeDirectoryMode = "0700";
        Restart = "always";
        RestartSec = 5;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };

    # launcher-say's pages held in memory, so a chord's program is never
    # read back from the disk, which took up to 1.7 s under load
    # (launcher-say/src/main.rs). vmtouch locks the file's pages and then
    # waits, waking for nothing; it holds well under the 8 MiB a user may
    # lock.
    systemd.user.services.launcher-say-resident = {
      Unit = {
        Description = "The launcher's chord program, held in memory";
        PartOf = [ "graphical-session.target" ];
        After = [ "graphical-session.target" ];
      };
      Service = {
        ExecStart = "${lib.getExe pkgs.vmtouch} -q -l ${lib.getExe launcherSay}";
        Restart = "on-failure";
        RestartSec = 1;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };

    # The shell's weight in the journal every ten minutes, as `shell weighs
    # N MB: JS heap ...`, so how it grows between switches and what a cut
    # saved are read there rather than from one look at a running shell. It
    # only reads /proc, the shell's cgroup and its GPU's fdinfo, in about
    # 30 ms, attaching to nothing. It runs only while the shell does
    # (Requisite), so a ten-minute mark with no shell, as at boot before
    # the session starts, skips it rather than leaving it failed: a boot
    # test spanning a mark failed on it (16V6F4, 2026-11-13).
    systemd.user.services.shell-weighs = {
      Unit = {
        Description = "Says what the shell weighs, part by part";
        Requisite = [ "quickshell.service" ];
        After = [ "quickshell.service" ];
      };
      Service = {
        Type = "oneshot";
        ExecStart = lib.getExe shellWeighs;
        Slice = "background.slice";
      };
    };
    systemd.user.timers.shell-weighs = {
      Unit.Description = "Weighs the shell every ten minutes";
      Timer = {
        OnCalendar = "*:0/10";
        AccuracySec = "1min";
      };
      Install.WantedBy = [ "timers.target" ];
    };

    # Hears the clipboard for the shell, which can't itself, and says what
    # it holds to every act on a copy (Copied.qml). While the clipboard
    # collection is on it keeps the copies for the launcher too: text in
    # memory, and pictures in the runtime directory, which is memory too.
    # Off, it keeps none. While Emoji is on it serves Emoji's rows, which
    # it pastes.
    systemd.user.services.clipboard-history = {
      Unit = {
        Description = "The clipboard, heard for the shell and kept in memory for the launcher";
        PartOf = [ "graphical-session.target" ];
        After = [ "graphical-session.target" ];
        Before = [ "quickshell.service" ];
      };
      Service = {
        ExecStart = lib.concatStringsSep " " (
          [
            (lib.getExe clipboardHistory)
            "--socket %t/clipboard-history.sock"
            # The launcher's words, which Copies' rows are said in.
            "--vocabulary ${launcherWords}"
          ]
          ++ (
            if keepsCopies then
              [
                "--keep ${toString launcher.providers.clipboard.keep}"
                "--pictures %t/clipboard-history"
              ]
            else
              [ "--keep 0" ]
          )
          ++ lib.optional listsEmoji "--emoji ${emojiListed}"
        );
        Restart = "on-failure";
        RestartSec = 1;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };

    # Where the laptop is and the zone the clock keeps, held over
    # $XDG_RUNTIME_DIR/daemons/location.sock in the shell's daemon interface
    # for Location.qml and clock-follows (whereabouts/src/main.rs says how).
    # Every machine runs it for the clock's zone; one that follows the
    # laptop has it ask GeoClue too, keeping its last here in its runtime
    # folder through a restart and Joel's pin in its state folder, which
    # Now's Location sets and takes away by asking it on the socket. Given
    # the launcher's words it serves Location's rows too, worded and ready
    # to draw. A GeoClue that refuses or goes is asked again a minute on,
    # while the socket goes on answering.
    systemd.user.services.location = {
      Unit = {
        Description = "Holds where the laptop is and the clock's zone ready for the shell";
        PartOf = [ "graphical-session.target" ];
        After = [
          "graphical-session.target"
          "location.socket"
        ];
        # Its socket is systemd's, handed to it as it starts.
        Requires = [ "location.socket" ];
        # Restarted after a switch reloads the units, it comes back as the
        # new one.
        X-SwitchMethod = "restart";
      };
      Service = {
        ExecStart = lib.concatStringsSep " " (
          [
            # Run by its name, so ps -ef reads location rather than a
            # store path (docs/daemons.md, Shown).
            "@${lib.getExe whereabouts}"
            "location"
            "%t/daemons/location.sock"
            "/"
          ]
          ++ lib.optionals followsTheLaptop [
            "%t/whereabouts/here.json"
            "%S/whereabouts/pin.json"
            "%S/clock-follows/moved.json"
          ]
          ++ [
            "--words"
            "${pkgs.writeText "words.json" (builtins.toJSON launcher.words)}"
          ]
        );
        Restart = "on-failure";
        RestartSec = 5;
      }
      // lib.optionalAttrs followsTheLaptop {
        StateDirectory = [
          "whereabouts"
          "clock-follows"
        ];
        RuntimeDirectory = "whereabouts";
        RuntimeDirectoryPreserve = "yes";
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };

    # Its socket, held by systemd whether or not the daemon runs, so the
    # shell asking while it starts, or starts again, waits on it rather
    # than finding no socket.
    systemd.user.sockets.location = {
      Unit.Description = "Where the shell asks where the laptop is";
      Socket = {
        ListenStream = "%t/daemons/location.sock";
        SocketMode = "0600";
        DirectoryMode = "0700";
      };
      Install.WantedBy = [ "sockets.target" ];
    };

    # The clock's zone follows where the laptop is (clock-follows/): it
    # watches `here:laptop` over whereabouts' socket and decides each time
    # it's told, never otherwise. What it did is kept in its state folder,
    # which whereabouts says beside here for Now (Location.qml), and Put It
    # Back starts its other run, from the launcher.
    systemd.user.services.clock-follows = lib.mkIf followsTheLaptop {
      Unit = {
        Description = "Moves the clock's zone to where the laptop is";
        PartOf = [ "graphical-session.target" ];
        After = [
          "graphical-session.target"
          "location.socket"
        ];
        Wants = [ "location.socket" ];
        X-SwitchMethod = "restart";
      };
      Service = {
        StateDirectory = "clock-follows";
        ExecStart = lib.concatStringsSep " " [
          (lib.getExe clockFollows)
          "follow"
          "%t/daemons/location.sock"
          "%S/clock-follows"
          timedatectl
        ];
        Restart = "on-failure";
        RestartSec = 5;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };
    systemd.user.services.clock-put-back = lib.mkIf followsTheLaptop {
      Unit.Description = "Puts the clock's zone back where it was before it followed the laptop";
      Service = {
        Type = "oneshot";
        StateDirectory = "clock-follows";
        ExecStart = "${lib.getExe clockFollows} put-back %S/clock-follows ${timedatectl}";
      };
    };
    # How the clock is kept, as chrony says it, for Time: systemd holds the
    # socket and starts the reader the first time the shell connects, and
    # the reader asks chronyd only while the shell watches.
    systemd.user.sockets.timekeeping = lib.mkIf keepsTime {
      Unit.Description = "Where the launcher asks how the clock is kept";
      Socket = {
        ListenStream = "%t/io.jitsusama.Timekeeping";
        FileDescriptorName = "varlink";
        SocketMode = "0600";
      };
      Install.WantedBy = [ "sockets.target" ];
    };
    # WARP's link, held by warp (modules/warp/src/serve.rs) for Network's
    # WARP, so the launcher starts no process to read WARP or to switch it.
    # systemd holds the socket and starts warp as the shell first connects;
    # it asks warp-cli, the one the providers' settings name, only while
    # WARP is shown.
    systemd.user.sockets.warp = lib.mkIf keepsWarp {
      Unit.Description = "Where the launcher asks how WARP's link stands";
      Socket = {
        ListenStream = "%t/daemons/warp.sock";
        SocketMode = "0600";
        DirectoryMode = "0700";
      };
      Install.WantedBy = [ "sockets.target" ];
    };
    systemd.user.services.warp = lib.mkIf keepsWarp {
      Unit = {
        Description = "Holds WARP's link for the launcher";
        # Restarted after a switch, it comes back as the new one, holding
        # nothing until the link is next watched.
        X-SwitchMethod = "restart";
        Requires = [ "warp.socket" ];
        After = [ "warp.socket" ];
      };
      Service = {
        ExecStart = "${launcher.providers.warp.warp} serve %t/daemons/warp.sock";
        # The network's shaper says what waits on the way through the VPN
        # while it runs (modules/shaper), so the Work Network says it too.
        Environment = [
          "WARP_SHAPER=/run/shaper/status.json"
        ]
        ++ lib.optional (
          launcher.providers.warp.warpCli != ""
        ) "WARP_CLI=${launcher.providers.warp.warpCli}";
        Restart = "on-failure";
        RestartSec = 1;
      };
    };
    # Joel's timers, kept and rung by their daemon whether or not the shell
    # is up (timers/src/main.rs). It runs from the start of the session, not
    # only once asked, so a timer kept through a reboot rings on time; its
    # socket is systemd's, so the shell asking while it starts again waits
    # on it rather than finding nothing there.
    systemd.user.sockets.timers = lib.mkIf keepsTimers {
      Unit.Description = "Where the launcher asks after Joel's timers";
      Socket = {
        ListenStream = "%t/daemons/timers.sock";
        SocketMode = "0600";
        DirectoryMode = "0700";
      };
      Install.WantedBy = [ "sockets.target" ];
    };
    systemd.user.services.timers = lib.mkIf keepsTimers {
      Unit = {
        Description = "Keeps Joel's timers and rings each as a notice";
        # Restarted after a switch, it comes back as the new one, every
        # timer read back from where it's kept.
        X-SwitchMethod = "restart";
        Requires = [ "timers.socket" ];
        After = [ "timers.socket" ];
      };
      Service = {
        StateDirectory = "timers";
        ExecStart = lib.concatStringsSep " " (
          [
            (lib.getExe timers)
            "%t/daemons/timers.sock"
            "%S/timers/held.json"
            "${timerWords}"
          ]
          ++ timerSound
        );
        Restart = "on-failure";
        RestartSec = 1;
      };
      Install.WantedBy = [ "default.target" ];
    };

    # Firmware's installs are put in by the machine daemon, with the
    # fwupdmgr Firmware names, so the shell spawns nothing; a machine
    # without fwupd names none, and installs nothing.
    systemd.user.services.machine.Service.Environment = lib.mkIf (
      (launcher.providers.firmware.fwupdmgr or "") != ""
    ) [ "MACHINE_FWUPDMGR=${launcher.providers.firmware.fwupdmgr}" ];

    systemd.user.services.timekeeping = lib.mkIf keepsTime {
      Unit = {
        Description = "Says how the clock is kept, for the launcher's Time";
        Requires = [ "timekeeping.socket" ];
        After = [ "timekeeping.socket" ];
      };
      Service = {
        ExecStart = lib.getExe timekeeping;
        Restart = "on-failure";
      };
    };

    # Keep Awake, from the launcher (KeepAwake.qml): one service holds
    # logind's lock against going idle and the other its lock against the
    # lid, on battery as on the charger, since Joel keeps it awake to run
    # with no power and the lid shut (2026-11-10). Neither holds the
    # lock against sleep: logind's own handling of the power and suspend keys
    # honours it, so a suspend Joel asks for with the power key would be
    # refused. (The launcher's suspend is his own and passes his own locks,
    # and the lid passes the sleep lock by logind's default, so neither would
    # be held back; logind v262, KP0L6A, 2026-11-10.)
    # Each leaves a file in the runtime directory while it runs, for the
    # shell to watch.
    systemd.user.services.keep-awake = holding {
      what = "idle";
      flag = "kept-awake";
      Description = "Keeps the machine from sleeping while work runs";
      PartOf = [ "graphical-session.target" ];
    };
    # Only where a shut lid suspends: elsewhere it never does, and holding
    # the lid would change nothing. A battery run nearly flat still
    # hibernates, as UPower has it (../power-profiles-daemon/nixos.nix),
    # since neither lock holds that back.
    systemd.user.services.keep-awake-lid = lib.mkIf lidSleeps (holding {
      what = "handle-lid-switch";
      flag = "kept-awake-lid";
      Description = "Keeps the machine running with its lid shut";
      PartOf = [ "keep-awake.service" ];
      After = [ "keep-awake.service" ];
      # The sleep a shut lid brought locked the screen first (../swayidle),
      # so while it's held a shut lid locks it instead, away from a desk.
      runs = [ (lib.getExe lidHeld) ];
    });

    # A switch that changes the shell's files left the new shell compiling
    # nearly all of them before it listened, so a chord waited 1.9 s from the
    # old shell's stop (12 November). Compiled once they're linked, while the
    # old shell still runs, the new one starts from the cache. Every QML file
    # the shell links is named, as its restart's triggers are.
    home.activation.quickshellStartsWarm =
      let
        isShells = name: lib.hasPrefix "quickshell/" name && lib.hasSuffix ".qml" name;
        linked = lib.attrNames config.xdg.configFile;
        warmed = lib.remove "quickshell/warm.qml" (lib.filter isShells linked);
        warms = pkgs.writeText "quickshell-warms.json" (
          builtins.toJSON (map (lib.removePrefix "quickshell/") warmed)
        );
      in
      lib.hm.dag.entryBetween [ "reloadSystemd" ] [ "linkGeneration" ] ''
        run ${lib.getExe startsWarm} ${warms}
      '';

    systemd.user.services.quickshell = {
      Unit = {
        Description = "Quickshell, which draws the launcher, notifications and lock screen";
        PartOf = [ "graphical-session.target" ];
        After = [ "graphical-session.target" ];
        # Quickshell watches its files but not the links Home Manager swaps
        # them by, so a switch would leave it drawing the old ones. Naming
        # every file here changes the unit when any of them changes, and the
        # list its start links says which it runs (quickshell-takes-new-files
        # below).
        X-Restart-Triggers = shellFiles;
        # A switch leaves the shell running, and quickshell-takes-new-files
        # restarts it once the launcher is idle. Restarted inside the switch,
        # it went whenever the switch's turn came: at 18:08 on 13 November,
        # 60 s after the launcher closed, as Joel opened it again. Stopped
        # with the switch's batch, it stayed down 91 s on 10 November while
        # disk-guard's stop ran to its timeout. Until its restart the old
        # shell runs with the new files linked under it, which is safe: it
        # made every surface and provider from its files as it started
        # (shell.qml, Launcher.qml, Facts.qml) and loads none later, so it
        # draws as it did until its restart brings the new ones.
        X-SwitchMethod = "keep-old";
      };
      Service = {
        # Quickshell turns every one of its own categories on at every level
        # for its log file, so each debug line, from every file read to every
        # PipeWire event, was made on the thread saying it and woke the
        # logging thread: two wakes a second of the locked shell's, for lines
        # nobody reads (2026-10-31). Without them its file keeps what's shown,
        # as the journal does.
        ExecStart = "${lib.getExe' pkgs.quickshell "quickshell"} --no-detailed-logs";
        # GTK's platform theme, which other Qt programs take, changes the
        # shell's default font and slows its start, and it draws from the
        # theme itself.
        UnsetEnvironment = [ "QT_QPA_PLATFORMTHEME" ];
        # niri calls back a surface it isn't drawing, as behind the lock or on
        # a dark screen, only once a second. Qt took a callback 100 ms late for
        # a window gone out of sight and drew it whole when it came, asking for
        # the next, so every window drew once a second while Joel's screen was
        # locked (2026-10-31). With no timeout a late callback changes nothing;
        # a frame drawn while one is owed still waits for it, a tenth of a
        # second at most.
        #
        # Quickshell allocates through jemalloc, which gives each of its 20 to
        # 30 threads an arena of its own (four for each CPU), and each arena
        # keeps the pages its thread freed for seconds before handing them
        # back. Held to one, the threads share what's freed: in the lab, on
        # Joel's records scrambled, the shell's memory outside its JavaScript
        # heap and its files sat at 92 to 127 MB through every place opened
        # and every search, against 114 to 141 with four arenas and 159 to
        # 202 with one for each thread, and its openings and keys took as
        # long as with four (2026-11-12). Four held Joel's lit shell at 150
        # to 171 MB, against 190 to 200 before, so one follows it.
        #
        # Quickshell watches every file it loaded, and each one's folder, to
        # reload when one changes: one inotify holding 194 watches on Joel's
        # shell (2026-11-12). Here they are store files, which never change,
        # and the unit is restarted onto new ones instead (its companion below),
        # so the watch is turned off. shell-preview and the nested niri
        # checks run the working copy outside this unit, and keep it.
        Environment = [
          "QT_WAYLAND_FRAME_CALLBACK_TIMEOUT=0"
          "MALLOC_CONF=narenas:1"
          "QS_DISABLE_FILE_WATCHER=1"
        ];
        # The bar, the launcher, notices and the lock screen are the
        # desktop's own, so they sit with the compositor in session.slice, at
        # its tenfold CPU weight and behind its memory floor (modules/slices),
        # rather than in app.slice beside Chrome and Slack, where a build's
        # pressure reaches them first.
        Slice = "session.slice";
        # Each file the shell watches is an inotify of its own, so Joel's lit
        # shell held 617 files, 430 of them watches, against systemd's soft
        # limit of 1024. Past the limit the kernel drops the files niri sends
        # along with its messages, and a shell 3 s after loading read a
        # format table with no file, lost its Wayland connection and died
        # (switch848, 2026-11-12, 21:41:35). Qt waits by poll, never select,
        # so a file past 1024 is safe in it.
        LimitNOFILE = "16384:524288";
        # The launcher's open mark says nothing once the shell is gone, so a
        # switch waiting on it never waits on a shell that died.
        ExecStopPost = "${lib.getExe' pkgs.coreutils "rm"} -f %t/quickshell-launcher";
        # Which files this shell started from, for quickshell-takes-new-files
        # to tell whether a switch changed them.
        ExecStartPost = "${lib.getExe' pkgs.coreutils "ln"} -sfn ${shellFilesList} %t/quickshell-files";
        # The launcher's socket goes before the shell is told to stop, so a
        # chord said as it stops finds no socket and waits for the next shell
        # (launcher-say/src/main.rs), rather than being heard by one that
        # dies before it opens: in the lab, a chord said as the stop began
        # was taken by the stopping shell and never opened (2026-11-12).
        ExecStop = "${lib.getExe' pkgs.coreutils "rm"} -f %t/launcher.sock";
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

    # Brings the shell onto a switch's files once the launcher and the talk
    # line have been shut 30 s, or past an hour at their next close, never
    # under Joel's hands (./takes-new-files.sh). Each
    # switch starts it once the units reload; a switch while it waits leaves
    # it waiting, since the restart it ends in takes the newest files.
    systemd.user.services.quickshell-takes-new-files = {
      Unit = {
        Description = "Restarts Quickshell onto a switch's files once the launcher is idle";
        After = [ "quickshell.service" ];
        X-SwitchMethod = "keep-old";
      };
      Service = {
        Type = "exec";
        ExecStart = "${lib.getExe pkgs.bash} ${./takes-new-files.sh} ${shellFilesList}";
        Environment = [
          "PATH=${
            lib.makeBinPath [
              pkgs.coreutils
              pkgs.gnugrep
              pkgs.systemd
            ]
          }"
        ];
        Slice = "session.slice";
      };
    };

    # Started without waiting, so the switch never waits on the launcher.
    # Nothing here fails a switch: without the account's systemd, as before
    # the first login, the shell takes its files as it starts.
    home.activation.quickshellTakesNewFiles = lib.hm.dag.entryAfter [ "reloadSystemd" ] ''
      run env XDG_RUNTIME_DIR="''${XDG_RUNTIME_DIR:-/run/user/$(id -u)}" \
        ${lib.getExe' pkgs.systemd "systemctl"} --user start --no-block \
        quickshell-takes-new-files.service || true
    '';
  };
}
