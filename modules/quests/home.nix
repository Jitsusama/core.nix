# Quests' rows, ready for the launcher: the quests daemon, a program of
# core's own (src/main.rs), follows the home realm's mycelium node for
# the open and ended quests and the sessions holding them, and makes
# every row Quests shows from them, in Quests' order. It serves them over
# $XDG_RUNTIME_DIR/daemons/quests.sock in the shell's daemon interface,
# each watch told only the rows that moved, so the launcher holds and
# sorts nothing of Quests (Joel, 2026-11-10: "the daemons need to be the
# data layer, and they need to serve the launcher"). It says them in the
# launcher's own words.json, so a word changes in one place.
#
# A machine with no realm runs no node and has no quests to show, so it
# runs no daemon, and the shell runs only on Linux.
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
  systemd.user.services.quests = {
    Unit = {
      Description = "Makes Quests' rows ready for the launcher";
      X-SwitchMethod = "restart";
      # Its socket is systemd's, handed to it as it starts.
      Requires = [ "quests.socket" ];
      After = [ "quests.socket" ];
    };
    Service = {
      ExecStart = lib.escapeShellArgs [
        (lib.getExe package)
        "--vocabulary"
        "${(import ../quickshell/launcher/words.nix).file}"
        "--node"
        "%t/mycelium/${realm}.sock"
        "--socket"
        "%t/daemons/quests.sock"
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
  systemd.user.sockets.quests = {
    Unit.Description = "Where the launcher asks for Quests' rows";
    Socket = {
      ListenStream = "%t/daemons/quests.sock";
      SocketMode = "0600";
      DirectoryMode = "0700";
    };
    Install.WantedBy = [ "sockets.target" ];
  };
}
