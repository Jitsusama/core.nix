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

  qml = [
    "shell.qml"
    "Prompt.qml"
    "KeyNames.js"
    "QueryKeys.js"
    "ModeMarks.js"
    "CommandLine.qml"
    "Commands.js"
    "Editing.js"
    "SetWords.js"
    "Ex.js"
    "CommandBinding.js"
    "ListedKeys.js"
    "Typing.js"
    "Bands.js"
    "Search.js"
    "Operators.js"
    "Finding.js"
    "Glide.js"
    "Confirm.js"
    "KeyHints.qml"
    "StatusLine.qml"
    "StatusLine.js"
    "Drawer.qml"
    "DrawerFigure.js"
    "DrawerPage.js"
    "Identifier.js"
    "Pictures.js"
    "Launcher.qml"
    "Choice.qml"
    "Ground.qml"
    "MarkSize.js"
    "Mark.qml"
    "Columns.js"
    "Gutter.qml"
    "ReadsOn.qml"
    "Wash.qml"
    "Reading.js"
    "Measures.js"
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
    "NoticeKeys.js"
    "LineEditing.js"
    "NoticeList.js"
    "Changes.js"
    "Ex.js"
    "HeldNotices.qml"
    "HeldTimers.qml"
    "Notifications.qml"
    "Music.qml"
    "Music.js"
    "Lock.qml"
    "PictureTime.qml"
    "Polkit.qml"
    "Askpass.qml"
    "PromptHead.qml"
    "PromptFoot.qml"
    "Hairline.qml"
    "Askpass.js"
    "Volume.qml"
    "Power.qml"
    "Brightness.qml"
    "Meter.qml"
    "Meters.qml"
    "Facts.qml"
    "Wallpaper.qml"
    "Lifted.qml"
    "OpenBelow.qml"
    "Niri.qml"
    "Wifi.qml"
    "ProcessorTelemetry.qml"
    "Timekeeping.qml"
    "Calendars.qml"
    "SignIn.qml"
    "Floating.qml"
    "Card.qml"
    "Cover.qml"
    "NoticeRow.qml"
    "NoticeBody.js"
    "Signature.qml"
    "ProgramGlyphs.js"
    "Fuzzy.js"
    "Picks.qml"
    "Trials.qml"
    "Traces.qml"
    "Words.qml"
    "Files.qml"
    "FileFacts.js"
    "When.js"
    "Taken.qml"
    "Copied.qml"
    "Copied.js"
    "Pasting.qml"
    "Pasting.js"
    "Notices.qml"
    "Forward.js"
    "Daemon.qml"
    "AgentSources.qml"
    "PiSource.qml"
    "PiSource.js"
    "Sessions.qml"
    "SessionWindows.js"
    "Places.js"
    "Location.qml"
    "WaitingOnYou.qml"
    "Waiting.js"
    "ForYouHolds.qml"
    "SpendWatch.qml"
    "SpendWatch.js"
    "Eero.qml"
    "Jobs.qml"
    "Jobs.js"
    "KeepAwake.qml"
    "Recorder.qml"
    "Picker.qml"
    "Picking.js"
    "Quiet.qml"
    "Lights.qml"
    "DeskLight.qml"
    "ConnectedKeyboards.qml"
    "KeyboardLayer.qml"
    "KeyboardLayout.qml"
    "KeyboardMap.qml"
    "Report.qml"
    "Reported.qml"
    "Frames.qml"
    "Counted.qml"
    "pieces/Drawers.js"
    "pieces/Holding.js"
    "pieces/Piece.qml"
    "pieces/StatusFile.qml"
    "launcher/Provider.qml"
    "launcher/Playing.qml"
    "launcher/Recents.qml"
    "launcher/Listed.qml"
    "launcher/Repositories.qml"
    "launcher/Repositories.js"
    "launcher/Previewer.qml"
    "launcher/Machine.js"
    "launcher/Heat.js"
    "launcher/Power.js"
    "launcher/Awake.js"
    "launcher/Traffic.js"
    "launcher/HomeNetwork.js"
    "launcher/HomeNetworkAddresses.js"
    "launcher/HomeNetworkDrawers.js"
    "launcher/HomeNetworkHistory.js"
    "launcher/Cameras.js"
    "launcher/Firmware.js"
    "launcher/Warp.js"
    "launcher/RealmMachines.js"
    "launcher/WarpSeen.js"
    "launcher/WarpSwitch.js"
    "launcher/Interfaces.js"
    "launcher/Networkd.js"
    "launcher/WifiLink.js"
    "launcher/Links.js"
    "launcher/Nearby.js"
    "launcher/Sounds.js"
    "launcher/Targets.js"
    "launcher/Plate.js"
    "launcher/Screens.js"
    "launcher/ScreenAnswers.js"
    "launcher/Backlight.js"
    "launcher/Numbers.js"
    "launcher/Addresses.js"
    "launcher/Subnets.js"
    "launcher/Characters.js"
    "launcher/Units.js"
    "launcher/Zones.js"
    "launcher/Moment.js"
    "launcher/Keeping.js"
    "launcher/Warmth.js"
    "launcher/Ago.js"
    "launcher/Timing.js"
    "launcher/Colours.js"
    "launcher/Asking.js"
    "launcher/Catalogue.js"
    "launcher/Rank.js"
    "launcher/Showing.js"
    "launcher/Seen.js"
    "launcher/Pacing.js"
    "launcher/Wanted.js"
    "launcher/Standing.js"
    "launcher/Steady.js"
    "launcher/RowsKept.js"
    "launcher/Order.js"
    "launcher/PlaceRow.js"
    "launcher/RowsPlace.js"
    "launcher/Becoming.js"
    "launcher/Recent.js"
    "launcher/Recents.js"
    "launcher/SessionRow.js"
    "launcher/SessionMade.js"
    "launcher/Landed.js"
    "launcher/SessionDrawer.js"
    "launcher/Holders.js"
    "launcher/SpendDrawer.js"
    "launcher/Trials.js"
    "launcher/Traces.js"
    "launcher/Emoji.js"
    "launcher/KeyGroups.js"
    "launcher/KeyLayers.js"
    "launcher/Keyboards.js"
    "launcher/KeyCounts.js"
    "launcher/Media.js"
    "launcher/NowGroups.js"
    "launcher/ForYou.js"
    "launcher/ForYouFilter.js"
    "launcher/Program.js"
    "launcher/HeldWindows.js"
    "launcher/Placed.js"
    "launcher/Captured.js"
    "launcher/NeedsYou.js"
    "launcher/Here.js"
    "launcher/Notice.js"
    "launcher/Lately.js"
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
          "--who=Keep Awake"
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
      key = if lib.hasSuffix "++" keys then "+" else lib.last (lib.splitString "+" keys);
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
        # Super+N takes the notices on screen, and opens For You's Desktop
        # when there are none (launcher/Now.qml's leadsTo), so the launcher
        # steps into it.
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

  # The program that hears every copy, for the clipboard collection.
  clipboardHistory = pkgs.callPackage ./clipboard/package.nix { };

  # The package the system installs warp-cli by, WARP's own client, found by
  # the program it says it is, or null where the system installs none.
  systemWarpCli = lib.findFirst (
    package: lib.isDerivation package && (package.meta.mainProgram or "") == "warp-cli"
  ) null (osConfig.environment.systemPackages or [ ]);
  keepsCopies = launcher.providers.clipboard.enable or false;

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
  # Whether shutting the lid on the charger suspends the machine, as logind
  # does unless told otherwise; the lid module tells it to ignore it. Only
  # where it suspends does keeping awake need to hold the lid too. Only
  # what's said for the charger counts, so a machine that ignores its lid
  # everywhere, as the test VM's does, still holds it, which changes
  # nothing there.
  lidSleepsOnPower =
    (osConfig.services.logind.settings.Login.HandleLidSwitchExternalPower or "suspend") != "ignore";
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
    default =
      config.jitsusama.pi.chime.want
        or "${pkgs.sound-theme-freedesktop}/share/sounds/freedesktop/stereo/message.oga";
    defaultText = lib.literalExpression ''config.jitsusama.pi.chime.want, or "''${pkgs.sound-theme-freedesktop}/share/sounds/freedesktop/stereo/message.oga" without pi'';
    description = ''
      The one sound the shell makes, at half volume, when a pi session
      first wants Joel, so he hears it from across the room, since it
      posts no notice; never while notices are held, nor for a want he's
      already looking at. Where the machine has pi it's pi's low bell
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
        # What asks logind to restart into the firmware's setup, by its
        # store path, so its refusal can be read and said.
        systemctl = lib.getExe' pkgs.systemd "systemctl";
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
      # Network. It holds nothing until the daemon has written its record.
      eero = {
        order = 102;
        source = "eero";
        weight = 0.9;
        enable = config.systemd.user.services ? eero-watch;
      };
      # The devices sound goes out of and comes in through.
      sound = {
        order = 103;
        source = "sound";
        weight = 0.9;
        # What reads a card's ports, what moves one onto another, and what
        # reads how a device keeps time while its drawer is out, a line at
        # a time through stdbuf, and what hears and sets where each
        # program's streams are sent.
        pwDump = lib.getExe' pkgs.pipewire "pw-dump";
        pwMetadata = lib.getExe' pkgs.pipewire "pw-metadata";
        pwTop = lib.getExe' pkgs.pipewire "pw-top";
        stdbuf = lib.getExe' pkgs.coreutils "stdbuf";
        wpctl = lib.getExe' pkgs.wireplumber "wpctl";
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
    home.packages = lib.optional followsTheLaptop whereabouts ++ [
      askpass
      launcherInATerminal
      launcherSay
      pkgs.quickshell
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
                    # What each connection is, from ip, and the words
                    # wifid says Wi-Fi's rows in.
                    // lib.optionalAttrs (provider.source == "networks") {
                      ip = lib.getExe' pkgs.iproute2 "ip";
                    }
                    // lib.optionalAttrs (provider.source == "wifi") {
                      words = "${launcherWords}";
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
              // Whether a shut lid suspends the machine on the charger.
              readonly property bool lidSleepsOnPower: ${lib.boolToString lidSleepsOnPower}
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
    # (2026-11-04). Started first, it's the instance; started beside one
    # already running, its hidden window keeps that one instead, and it
    # waits on that window either way, so the unit stays up. It runs in
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
          "-1 --instance-group pi-sessions --wait-for-single-instance-window-close"
          "-o sync_to_monitor=yes"
          "--class pi-sessions-kept --start-as=hidden ${lib.getExe' pkgs.coreutils "sleep"} infinity"
        ];
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

    # Where the laptop is and the zone the clock keeps, held over
    # $XDG_RUNTIME_DIR/daemons/location.sock in the shell's daemon interface
    # for Location.qml and clock-follows (whereabouts/src/main.rs says how).
    # Every machine runs it for the clock's zone; one that follows the
    # laptop has it ask GeoClue too, keeping its last here in its runtime
    # folder through a restart and Joel's pin in its state folder, which
    # Now's Location sets and takes away through the same program, on the
    # shell's path. A GeoClue that refuses or goes is asked again a minute
    # on, while the socket goes on answering.
    systemd.user.services.whereabouts = {
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
            (lib.getExe whereabouts)
            "%t/daemons/location.sock"
            "/"
          ]
          ++ lib.optionals followsTheLaptop [
            "%t/whereabouts/here.json"
            "%S/whereabouts/pin.json"
            "%S/clock-follows/moved.json"
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
    # than finding no socket. It's named for its domain, so it names the
    # daemon it hands its listener to, which systemd would otherwise look
    # for as location.service and refuse.
    systemd.user.sockets.location = {
      Unit.Description = "Where the shell asks where the laptop is";
      Socket = {
        Service = "whereabouts.service";
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
        Environment = lib.optional (
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
    # lid, which the shell takes only while there's power. Neither holds the
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
    # Only where a shut lid on the charger suspends: elsewhere it never does,
    # and holding the lid would change nothing.
    systemd.user.services.keep-awake-lid = lib.mkIf lidSleepsOnPower (holding {
      what = "handle-lid-switch";
      flag = "kept-awake-lid";
      Description = "Keeps the machine running with its lid shut, while there's power";
      PartOf = [ "keep-awake.service" ];
      # Bound to the mains supply, which systemd holds ready only while it's
      # online (nixos.nix), so it stops the moment the power goes, whatever
      # the shell heard; it never starts without it.
      BindsTo = [ "sys-subsystem-power_supply-mains.device" ];
      After = [
        "keep-awake.service"
        "sys-subsystem-power_supply-mains.device"
      ];
      ConditionACPower = true;
    });

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
        # Without a method, a switch stops a changed unit with every other it
        # stops, waits for the slowest, and only then starts it again: the
        # shell stayed down 91 s on 10 November while disk-guard's stop ran
        # to its timeout, with no launcher, notices or lock client. Restarted,
        # it's left out of that batch and runs on until its own restart after
        # the units reload, which waits on nothing else. Until the batch ends
        # the old shell runs with the new files already linked under it, which
        # is safe: it made every surface and provider from its files as it
        # started (shell.qml, Launcher.qml, Facts.qml) and loads none later, so
        # it draws as it did until its restart brings the new ones.
        X-SwitchMethod = "restart";
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
        Environment = [ "QT_WAYLAND_FRAME_CALLBACK_TIMEOUT=0" ];
        # The bar, the launcher, notices and the lock screen are the
        # desktop's own, so they sit with the compositor in session.slice, at
        # its tenfold CPU weight and behind its memory floor (modules/slices),
        # rather than in app.slice beside Chrome and Slack, where a build's
        # pressure reaches them first.
        Slice = "session.slice";
        # The launcher's open mark says nothing once the shell is gone, so a
        # switch waiting on it never waits on a shell that died.
        ExecStopPost = "${lib.getExe' pkgs.coreutils "rm"} -f %t/quickshell-launcher";
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
