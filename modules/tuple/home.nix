# Tuple, for pairing. On Linux Tuple is a command and a daemon: the daemon
# holds the account, rings with a window of its own when someone calls and
# draws the other person's cursor over a shared screen, and the tuple command
# starts, joins and ends calls. A service starts the daemon with the desktop,
# so a call reaches Joel without his starting anything first. Tuple draws its
# own windows, which no theme reaches, and through X11, so every one of them
# has the same name to niri and none can be told apart by a window rule.
#
# Tuple shares a screen through the desktop's portal, the one way to on a
# Wayland desktop; its other ways only reach X11 windows. Tuple keeps each
# setting in a file of its own and tuple set rewrites it, so the switch
# writes the portal's in rather than linking it.
#
# Tuple writes a desktop entry each time its daemon starts, one that runs
# tuple with nothing after it, which only prints the command's help. This
# entry takes its place, hidden, since there's no window to open.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  tuple = lib.getExe pkgs.tuple;
  settings = "${config.xdg.dataHome}/tuple/0/settings";
in
{
  home.packages = [ pkgs.tuple ];

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
      Exec=${tuple} on
      Icon=tuple
      StartupWMClass=Tuple
      NoDisplay=true
    '';
  };
}
