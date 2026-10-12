# Tuple, for pairing. On Linux Tuple is a command and a daemon: the daemon
# holds the account, rings with a window of its own when someone calls and
# draws the other person's cursor over a shared screen, and the tuple command
# starts, joins and ends calls. A service starts the daemon with the desktop,
# so a call reaches Joel without his starting anything first. Tuple draws its
# own windows, which no theme reaches, and through X11, where each has a name
# of its own (tuple-main, tuple-error, tuple-incoming-call, tuple-screen),
# but xwayland-satellite passes niri only the class they share, Tuple, and
# none has a title, so no window rule can tell one from another.
#
# Most of them are small, the ring, an error, Tuple's own panel, and tiling
# would stretch each to the screen's height, so they all open floating, at
# the size Tuple asks for; the other person's screen, when they share it,
# does too, and tiles or fills the screen with niri's own keys. The cursor
# Tuple draws over a shared screen is a layer of its own over the whole
# screen, which no window rule reaches, and none is kept out of a
# screencast, since niri draws what it keeps out as black, and over that
# layer the whole share would be.
#
# A call link opened in a browser opens a page that hands the call to
# tuple://join?call_url_slug=<room>, which Tuple's Linux command doesn't
# take, so the desktop entry below does: it joins the room as tuple join
# would its link.
#
# Tuple shares a screen through the desktop's portal, the one way to on a
# Wayland desktop; its other ways only reach X11 windows. Tuple keeps each
# setting in a file of its own and tuple set rewrites it, so the switch
# writes the portal's in rather than linking it.
#
# Tuple writes a desktop entry each time its daemon starts, one that runs
# tuple with nothing after it, which only prints the command's help. This
# entry takes its place, hidden, since there's no window to open, and opens
# Tuple's links. A browser asks xdg-open, which takes the one entry that
# says it opens them when no list names one, so no list of defaults, which
# programs write too, needs to.
#
# The shell is where a call shows (docs/design.md, Calls): a piece in the
# margin while Joel's in one or one rings, with leaving, sharing and muting
# in its panel, the launcher's rows to open his room, join a call whose link
# was copied, and leave, and notices as people come and go, which Tuple on
# Linux doesn't give yet. Tuple.qml follows the daemon's log for them all.
{ niri, quickshell }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  tuple = lib.getExe pkgs.tuple;
  settings = "${config.xdg.dataHome}/tuple/0/settings";

  # Opens a tuple: link, or starts the daemon when given none. A room's
  # name is letters, digits, - and _, as Tuple's links write it, and
  # anything else is said rather than run.
  opens = pkgs.writeShellScript "tuple-opens" ''
    link=''${1:-}
    if [ -z "$link" ]; then
      exec ${tuple} on
    fi
    set -f
    room=""
    case "$link" in
      tuple://join\?*)
        IFS='&'
        for pair in ''${link#*\?}; do
          case "$pair" in
            call_url_slug=*) room=''${pair#call_url_slug=} ;;
          esac
        done
        ;;
    esac
    case "$room" in
      "" | *[!A-Za-z0-9_-]*) ;;
      *) exec ${tuple} join "https://tuple.app/c/$room" ;;
    esac
    exec ${lib.getExe' pkgs.libnotify "notify-send"} --app-name=Tuple \
      "Tuple can't open that link" "$link"
  '';
in
{
  imports = [
    niri
    quickshell
  ];

  home.packages = [ pkgs.tuple ];

  xdg.configFile = {
    "quickshell/Tuple.qml".source = ./Tuple.qml;
    "quickshell/Tuple.js".source = ./Tuple.js;
  };
  jitsusama.quickshell = {
    overview.pieces.call = {
      # Before the sharing, which a call often brings.
      order = 78;
      source = ./Call.qml;
      label = "call";
      tier = "when";
      mark = "call";
    };
    # Beside the desktop's actions, since each is one.
    launcher.providers.tuple = {
      order = 45;
      source = ./Calls.qml;
    };
  };

  systemd.user.services.tuple = {
    Unit = {
      Description = "Tuple's daemon, which takes calls";
      PartOf = [ "graphical-session.target" ];
      After = [ "graphical-session.target" ];
    };
    Service = {
      # tuple on starts the daemon behind it and returns.
      Type = "forking";
      ExecStart = "${tuple} on";
      ExecStop = "${tuple} off";
      Restart = "on-failure";
    };
    Install.WantedBy = [ "graphical-session.target" ];
  };

  # Tuple reads the value only as it writes it, with no newline after it.
  home.activation.tupleSharesThroughThePortal = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    run mkdir -p ${lib.escapeShellArg settings}
    run sh -c 'printf portal > "$1"' sh ${lib.escapeShellArg "${settings}/capture"}
  '';

  home.file."${config.xdg.dataHome}/applications/tuple.desktop" = {
    force = true;
    text = ''
      [Desktop Entry]
      Type=Application
      Name=Tuple
      Comment=Pair programming
      Exec=${opens} %u
      Icon=tuple
      StartupWMClass=Tuple
      MimeType=x-scheme-handler/tuple;
      NoDisplay=true
    '';
  };

  # After niri's own rules, which match no window of Tuple's.
  xdg.configFile."niri/config.kdl".text = lib.mkAfter ''
    window-rule {
        match app-id="^Tuple$"
        open-floating true
    }
  '';
}
