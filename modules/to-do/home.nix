# What clears each of Joel's to-dos by itself: the to-do daemon, a program
# of core's own (src/main.rs), runs each to-do's check and hears its
# signals, and writes what it found to ~/.local/state/mission/verdicts.json,
# which the harness settles each to-do by and the mycelium node serves to
# For You. Every session that needs him writes its to-dos beside it, in
# mission/waiting, through mesh_wait.
#
# A check runs with the user manager's environment as it is when the
# check runs, read through systemctl, the one every session's shell
# starts from. The daemon starts before the desktop imports the
# session's PATH (~/.local/bin among it), so a to-do's check finds quick,
# ~/.local/bin and the wrappers as a session's would only by reading it
# afresh; the daemon's own git, journalctl and systemctl are named
# outright instead. A check runs only for a session the harness says is
# outside every sandbox, since here it runs as Joel. Each is stopped, with
# all it started, after 90 seconds, and nothing it started outlives it. A
# switch's core is read from update-to-main's follower's journal, and whether it's past a commit
# from a bare clone of core of the daemon's own, fetched only while a
# to-do waits on a commit it lacks. A switch restarts it, so it runs the
# daemon it was switched to; what it found done stays done.
#
# It holds what waits on Joel for the shell too, over
# $XDG_RUNTIME_DIR/daemons/to-do.sock in the shell's daemon interface:
# each to-do worded for For You, with what it found and Joel's word on it
# from mission/settled.json, which it keeps as he says it, each look the
# agents left him in the harness's looks folder, and all of it counted,
# for the bar's For You mark.
#
# It reads Linux's inotify, /proc and sysfs, so it runs only there.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };
  mission = "${config.xdg.stateHome}/mission";
  # The launcher's words, where the shell's module is imported, so each
  # to-do reaches For You worded; without it they're served unworded.
  words = config.jitsusama.quickshell.launcher.words or null;
in
lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
  home.packages = [ package ];

  systemd.user.services.to-do = {
    Unit = {
      Description = "Runs each to-do's check and hears its signals";
      X-SwitchMethod = "restart";
      # Its socket is systemd's, handed to it as it starts.
      Requires = [ "to-do.socket" ];
      After = [ "to-do.socket" ];
    };
    Service = {
      ExecStart = lib.escapeShellArgs (
        [
          (lib.getExe package)
          "${mission}/waiting"
          "${mission}/verdicts.json"
          "${mission}/signins"
          "${config.xdg.stateHome}/to-do/core.git"
          "--git"
          (lib.getExe pkgs.git)
          "--journalctl"
          "${pkgs.systemd}/bin/journalctl"
          "--systemctl"
          "${pkgs.systemd}/bin/systemctl"
          "--socket"
          "%t/daemons/to-do.sock"
          "--looks"
          "${config.xdg.stateHome}/pi/agentic-harness.pi/looks"
          "--settled"
          "${mission}/settled.json"
        ]
        ++ lib.optionals (words != null) [
          "--words"
          (pkgs.writeText "words.json" (builtins.toJSON words))
        ]
      );
      Restart = "always";
      RestartSec = 5;
      Slice = "background.slice";
    };
    Install.WantedBy = [ "default.target" ];
  };

  # Its socket, held by systemd whether or not the daemon runs, so the
  # shell asking while it starts, or starts again, waits on it rather than
  # finding no socket.
  systemd.user.sockets.to-do = {
    Unit.Description = "Where the shell asks what waits on Joel";
    Socket = {
      ListenStream = "%t/daemons/to-do.sock";
      SocketMode = "0600";
      DirectoryMode = "0700";
    };
    Install.WantedBy = [ "sockets.target" ];
  };
}
