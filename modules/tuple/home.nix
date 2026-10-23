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
# take, so the desktop entry below does: it joins the room as tuple call
# join would its link.
#
# Tuple shares a screen through the desktop's portal, the one way to on a
# Wayland desktop; its other ways only reach X11 windows. Tuple keeps each
# setting in a file of its own and tuple set rewrites it, so the switch
# writes the portal's in rather than linking it.
#
# Tuple writes a desktop entry each time its daemon starts, one that runs
# tuple with nothing after it, which only prints the command's help. This
# entry takes its place, hidden, since there's no window to open, and opens
# Tuple's links. xdg-open finds the entry by what it says it opens, but gio,
# and the desktop's portal that programs hand links to, find an app in a
# folder only through the folder's index or a list of defaults, and this
# folder has neither. So the entry is named the default in niri's own list,
# which they read before the account's mimeapps.list, the browser's to
# rewrite.
#
# The shell is where a call shows (docs/design.md, Calls): a piece in the
# margin while Joel's in one or one rings, with leaving, sharing and muting
# in its panel, the launcher's rows to open his room, join a call whose link
# was copied, and leave, and notices as people come and go, which Tuple on
# Linux doesn't give yet. Tuple.qml follows the daemon's log for them all.
#
# The tuple command completes in zsh through Tuple's own completion, which
# its command writes, so it knows every command and flag the version has.
#
# Tuple's daemon writes its log afresh each time it starts, so the service
# keeps the last three before it does: the log of whatever went wrong in a
# call is still there after a restart.
#
# Tuple is newer than nixpkgs' (./package.nix).
{ niri, quickshell }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };
  tuple = lib.getExe package;
  data = "${config.xdg.dataHome}/tuple/0";
  settings = "${data}/settings";

  # Tuple's people for the shell (people.rs): who's in Tuple's contacts and
  # how they stand, said only as it changes, since Tuple prints all of
  # them, over a megabyte, on every change of anyone's presence.
  people = pkgs.callPackage ../quickshell/pi-helper.nix {
    name = "tuple-people";
    source = ./people.rs;
  };

  # Opens a tuple: link, or starts the daemon when given none. A room's
  # name is letters, digits, - and _, as Tuple's links write it, and
  # anything else is said rather than run.
  opens = pkgs.writeShellScript "tuple-opens" ''
    link=''${1:-}
    if [ -z "$link" ]; then
      exec ${tuple} start
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
      *) exec ${tuple} call join "https://tuple.app/c/$room" ;;
    esac
    exec ${lib.getExe' pkgs.libnotify "notify-send"} --app-name=Tuple \
      "Tuple can't open that link" "$link"
  '';

  # zsh's completion for the tuple command, as Tuple's command writes it,
  # which needs only somewhere to keep its files. The command runs behind
  # nixpkgs' wrapper as .tuple-wrapped and writes the completion for that
  # name, which zsh would never use for tuple, so it's named for tuple.
  completion = pkgs.runCommandLocal "tuple-zsh-completion" { } ''
    export HOME=$PWD XDG_RUNTIME_DIR=$PWD/run
    mkdir -m 700 run
    mkdir -p $out/share/zsh/site-functions
    ${tuple} completion zsh | sed 's/\.tuple-wrapped/tuple/g' > $out/share/zsh/site-functions/_tuple
    grep -q '^#compdef tuple$' $out/share/zsh/site-functions/_tuple
  '';

  # Keeps the daemon's last three logs before it writes a new one; an empty
  # log is nothing to keep, so it leaves the others where they are.
  keepsLogs = pkgs.writeShellScript "tuple-keeps-logs" ''
    cd ${lib.escapeShellArg data} 2>/dev/null || exit 0
    [ -s log.txt ] || exit 0
    for n in 2 1; do
      if [ -e log.$n.txt ]; then mv -f log.$n.txt log.$((n + 1)).txt; fi
    done
    mv -f log.txt log.1.txt
  '';
in
{
  imports = [
    niri
    quickshell
  ];

  home.packages = [
    package
    completion
    people
  ];

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
    # Tuple's people, to call or join, found by name or email.
    launcher.providers.people = {
      order = 46;
      source = ./People.qml;
      weight = 0.9;
    };
  };

  systemd.user.services.tuple = {
    Unit = {
      Description = "Tuple's daemon, which takes calls";
      PartOf = [ "graphical-session.target" ];
      After = [ "graphical-session.target" ];
    };
    Service = {
      # tuple start starts the daemon behind it and returns.
      Type = "forking";
      ExecStartPre = "${keepsLogs}";
      ExecStart = "${tuple} start";
      ExecStop = "${tuple} stop";
      Restart = "on-failure";
      # A call is real time: its sound, camera and screen are encoded and
      # sent as they happen, so under load the daemon comes before the
      # other apps it shares app.slice with, ten to one, and at the most
      # the account may raise it. It stays out of session.slice, whose
      # memory floor the compositor and sound need: the daemon has peaked
      # at 935 MiB in a call, past that floor on its own.
      CPUWeight = 1000;
      Nice = -10;
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

  # niri's own list of defaults, which only a desktop's may hold; the
  # browser keeps rewriting the account's mimeapps.list.
  xdg.configFile."niri-mimeapps.list".text = ''
    [Default Applications]
    x-scheme-handler/tuple=tuple.desktop
  '';

  # After niri's own rules, which match no window of Tuple's.
  xdg.configFile."niri/config.kdl".text = lib.mkAfter ''
    window-rule {
        match app-id="^Tuple$"
        open-floating true
    }
  '';
}
