# The jobs broker, for pi's fenced commands: the agent fence hides the
# session bus, so a command can't reach the account's systemd, and the
# broker answers the two things they asked it for, on the one socket the
# fence lets through for it, %t/daemons/jobs-broker.sock. It makes a scope
# for the process asking, known by the kernel's pidfd for it, only into
# agents' own work, and says how units stand; it starts, stops and changes
# nothing else (src/main.rs says why and how). Each scope made and each
# refused is a line in the journal.
#
# Its command, `jobs-broker run -- COMMAND` and `jobs-broker status UNIT`,
# is on the path for agents to use. systemd's user manager runs only on
# Linux.
{ lib, pkgs, ... }:
let
  package = pkgs.callPackage ./package.nix { };
in
{
  config = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
    systemd.user.services.jobs-broker = {
      Unit = {
        Description = "Makes fenced pi commands' own scopes and says how units stand";
        X-SwitchMethod = "restart";
        # Its socket is systemd's, handed to it as it starts.
        Requires = [ "jobs-broker.socket" ];
        After = [ "jobs-broker.socket" ];
      };
      Service = {
        # Started as @<program> jobs-broker, so ps reads its name rather
        # than a store path, as docs/daemons.md says every daemon is.
        ExecStart = lib.escapeShellArgs [
          "@${lib.getExe package}"
          "jobs-broker"
          "serve"
          "--socket"
          "%t/daemons/jobs-broker.sock"
        ];
        Restart = "always";
        RestartSec = 1;
        Slice = "background.slice";
      };
      Install.WantedBy = [ "default.target" ];
    };

    # Its socket, held by systemd whether or not the broker runs, so a
    # command asking while it starts waits on it rather than finding none.
    systemd.user.sockets.jobs-broker = {
      Unit.Description = "Where fenced pi commands ask the jobs broker";
      Socket = {
        ListenStream = "%t/daemons/jobs-broker.sock";
        SocketMode = "0600";
        DirectoryMode = "0700";
      };
      Install.WantedBy = [ "sockets.target" ];
    };

    home.packages = [ package ];
  };
}
