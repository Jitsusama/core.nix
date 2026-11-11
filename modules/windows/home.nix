# The windows daemon: niri's windows, each named by its program and
# placed as its row says it, every workspace, and what holds the keys,
# held ready over $XDG_RUNTIME_DIR/daemons/windows.sock in the shell's
# daemon interface, for the launcher's Windows, Capture and each
# program's windows (src/main.rs says how). The shell imports it, since
# those are what it holds.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  windows = pkgs.callPackage ./package.nix { };
  # The launcher's vocabulary and Screens' number settings, which Screens'
  # ready rows are said in, and the machine daemon's socket (modules/machine),
  # where a screen's drawer reads its light while it's looked at; none where
  # the shell isn't imported, and then it makes no rows.
  launcher = config.jitsusama.quickshell.launcher or null;
  displays = if launcher == null then { } else launcher.providers.displays or { };
  worded = lib.optionals (launcher != null) [
    "--words"
    (pkgs.writeText "words.json" (builtins.toJSON launcher.words))
    "--numbers"
    (pkgs.writeText "numbers.json" (
      builtins.toJSON {
        group = displays.group or "";
        point = displays.point or "";
      }
    ))
    "--machine"
    "%t/daemons/machine.sock"
  ];
in
{
  # It isn't in background.slice: what it holds is what the launcher
  # shows as Joel opens it. It's part of the graphical session, whose
  # environment names niri's socket and where the desktop entries are.
  systemd.user.services.windows = {
    Unit = {
      Description = "Holds niri's windows ready for the shell";
      PartOf = [ "graphical-session.target" ];
      After = [
        "graphical-session.target"
        "windows.socket"
      ];
      # Restarted after a switch reloads the units, it comes back as the
      # new one, reading the new entries.
      X-SwitchMethod = "restart";
      Requires = [ "windows.socket" ];
    };
    Service = {
      ExecStart = lib.concatStringsSep " " (
        [
          (lib.getExe windows)
          "%t/daemons/windows.sock"
        ]
        ++ worded
      );
      Restart = "on-failure";
      RestartSec = 5;
    };
    Install.WantedBy = [ "graphical-session.target" ];
  };

  # Its socket, held by systemd whether or not the daemon runs, so the
  # shell asking while it starts waits on it rather than finding none.
  systemd.user.sockets.windows = {
    Unit.Description = "Where the shell asks the windows daemon";
    Socket = {
      ListenStream = "%t/daemons/windows.sock";
      SocketMode = "0600";
      DirectoryMode = "0700";
    };
    Install.WantedBy = [ "sockets.target" ];
  };
}
