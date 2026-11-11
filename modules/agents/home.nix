# Agents' rows, ready for the launcher: the agents daemon, a program of
# core's own (src/main.rs), follows the home realm's mycelium node for
# every pi session and the machine's queue, the to-do daemon for what
# waits on Joel, the windows daemon for when each session's window was
# last used, spend-watch's collection for what each spent, and the
# harness's list of orchestrators, and makes every row Agents shows from
# all of it, worded from the launcher's own words.json. It serves the
# sessions there are to reopen too, from the harness's restore.json,
# keeping what Joel said of them in a file of its own. It serves them
# over $XDG_RUNTIME_DIR/daemons/agents.sock in the shell's daemon
# interface, each watch told only the rows that moved, so the launcher
# holds and makes nothing of Agents (Joel, 2026-11-10: "the daemons need
# to be the data layer, and they need to serve the launcher").
#
# A machine with no realm runs no node and has no sessions to show, so it
# runs no daemon. It reads Linux's inotify, so it runs only there.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };
  realm = config.jitsusama.mycelium.realm or null;
in
lib.mkIf (pkgs.stdenv.hostPlatform.isLinux && realm != null) {
  systemd.user.services.agents = {
    Unit = {
      Description = "Makes Agents' rows ready for the launcher";
      X-SwitchMethod = "restart";
      # Its socket is systemd's, handed to it as it starts.
      Requires = [ "agents.socket" ];
      After = [ "agents.socket" ];
    };
    Service = {
      ExecStart = lib.escapeShellArgs [
        (lib.getExe package)
        "--vocabulary"
        "${../quickshell/launcher/words.json}"
        "--node"
        "%t/mycelium/${realm}.sock"
        "--to-do"
        "%t/daemons/to-do.sock"
        "--windows"
        "%t/daemons/windows.sock"
        "--spend"
        "%t/spend-watch/collection.json"
        "--orchestrators"
        "${config.xdg.stateHome}/pi/agentic-harness.pi/quest-workflow/orchestrators.json"
        "--restore"
        "${config.xdg.stateHome}/pi/restore.json"
        "--kept"
        "${config.xdg.stateHome}/agents/reopen.json"
        "--socket"
        "%t/daemons/agents.sock"
      ];
      Restart = "always";
      RestartSec = 5;
      Slice = "background.slice";
    };
    Install.WantedBy = [ "default.target" ];
  };

  # Its socket, held by systemd whether or not the daemon runs, so the
  # shell asking while it starts, or starts again, waits on it rather than
  # finding no socket.
  systemd.user.sockets.agents = {
    Unit.Description = "Where the launcher asks for Agents' rows";
    Socket = {
      ListenStream = "%t/daemons/agents.sock";
      SocketMode = "0600";
      DirectoryMode = "0700";
    };
    Install.WantedBy = [ "sockets.target" ];
  };
}
