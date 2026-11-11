# The machine daemon: how this machine is doing, its processor, memory,
# disks, boot and the programs using it most, held ready over
# $XDG_RUNTIME_DIR/daemons/machine.sock in the shell's daemon interface,
# for the launcher's Machine place and anything else that asks
# (src/main.rs says how). The shell imports it, since what Machine shows
# is what it holds.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  machine = pkgs.callPackage ./package.nix { };
  # The launcher's vocabulary and Heat's number settings, which Heat's
  # ready rows are said in; none where the shell isn't imported, and then
  # it makes no rows.
  launcher = config.jitsusama.quickshell.launcher or null;
  heat = if launcher == null then { } else launcher.providers.heat or { };
  worded = lib.optionals (launcher != null) [
    "--words"
    (pkgs.writeText "words.json" (builtins.toJSON launcher.words))
    "--numbers"
    (pkgs.writeText "numbers.json" (
      builtins.toJSON {
        group = heat.group or "";
        point = heat.point or "";
      }
    ))
  ];
in
{
  # It isn't in background.slice: what it holds is what the launcher shows
  # as Joel opens it, so it answers ahead of the work behind. It reads
  # nothing while nobody watches, so it costs nothing idle; what it hears
  # from its start, a sleep, Keep Awake's files, the chosen profile and
  # power-profiles-daemon, wakes it only as they change.
  systemd.user.services.machine = {
    Unit = {
      Description = "Holds how this machine is doing ready for the shell";
      # Restarted after a switch reloads the units, it comes back as the
      # new one.
      X-SwitchMethod = "restart";
      # Its socket is systemd's, handed to it as it starts.
      Requires = [ "machine.socket" ];
      After = [ "machine.socket" ];
    };
    Service = {
      ExecStart = lib.concatStringsSep " " (
        [
          (lib.getExe machine)
          "%t/daemons/machine.sock"
        ]
        ++ worded
      );
      Restart = "on-failure";
      RestartSec = 5;
    };
    Install.WantedBy = [ "default.target" ];
  };

  # Its socket, held by systemd whether or not the daemon runs, so the
  # shell asking while it starts, or starts again, waits on it rather than
  # finding no socket; asking starts it, should it have stopped.
  systemd.user.sockets.machine = {
    Unit.Description = "Where the shell asks the machine daemon";
    Socket = {
      ListenStream = "%t/daemons/machine.sock";
      SocketMode = "0600";
      DirectoryMode = "0700";
    };
    Install.WantedBy = [ "sockets.target" ];
  };
}
