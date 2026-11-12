# The desktop's notices, held by a daemon of their own: the notices
# daemon, a program of core's own (src/main.rs), is the session bus's
# notifications server, keeps every notice and its history, and serves
# each to the shell over $XDG_RUNTIME_DIR/daemons/notices.sock in the
# shell's daemon interface, as one record its cards and the launcher's
# rows read, so the launcher holds no notice of its own (Joel,
# 2026-11-10: "the daemons need to be the data layer, and they need to
# serve the launcher"). It judges each sender by the levels the shell's
# senders option sets, and holds a notice back while Quiet's file says so.
#
# Only one server may hold the notifications name, and it never takes the
# name from another, so a shell still serving it as a switch lands keeps
# it until it goes, the daemon starting again each second meanwhile. A
# notice sent while the daemon is down starts it, since the bus knows
# the unit by the name (the D-Bus service file below), and the bus holds
# the notice until it's up, so none is lost.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };
  # Each sender's level by its name in lower case, as the shell reads
  # them; an account without the shell sets none, and every sender
  # reaches a card.
  levels = pkgs.writeText "notices-levels.json" (
    builtins.toJSON (
      lib.mapAttrs' (name: sender: lib.nameValuePair (lib.toLower name) sender.level) (
        config.jitsusama.quickshell.notices.senders or { }
      )
    )
  );
  # The launcher's vocabulary, which each notice's rows are said in, as
  # the machine has it; its own words where it has no shell.
  launcher = config.jitsusama.quickshell.launcher or null;
  words = pkgs.writeText "notices-words.json" (
    builtins.toJSON (
      if launcher == null then (import ../quickshell/launcher/words.nix).value else launcher.words
    )
  );
  # Each program's own mark, so a notice wears the mark its program's row
  # does.
  marks = import ../quickshell/program-marks.nix { inherit lib pkgs launcher; };
in
lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
  systemd.user.services.notices = {
    Unit = {
      Description = "Holds the desktop's notices and serves them to the shell";
      X-SwitchMethod = "restart";
      # Its socket is systemd's, handed to it as it starts.
      Requires = [ "notices.socket" ];
      After = [ "notices.socket" ];
      # Starting again each second while another holds the name would pass
      # systemd's default limit of five starts in ten seconds, failing the
      # unit and its socket, so no limit is kept: it never gives up.
      StartLimitIntervalSec = 0;
    };
    Service = {
      ExecStart = lib.escapeShellArgs [
        (lib.getExe package)
        "--socket"
        "%t/daemons/notices.sock"
        "--state"
        "%S/notices"
        "--levels"
        "${levels}"
        "--senders"
        "${./senders.json}"
        "--quiet"
        "%t/quiet"
        "--vocabulary"
        "${words}"
        "--marks"
        "${marks}"
        "--shells"
        "%S/quickshell/by-shell"
      ];
      # A notice sent while it's down waits on the bus for it, so it comes
      # straight back.
      Restart = "always";
      RestartSec = 1;
    };
  };

  # Its socket, held by systemd from the start of the session whether or
  # not the daemon runs, so the shell asking while it starts, or starts
  # again, waits on it rather than finding no socket, and starts it.
  systemd.user.sockets.notices = {
    Unit.Description = "Where the shell asks for the desktop's notices";
    Socket = {
      ListenStream = "%t/daemons/notices.sock";
      SocketMode = "0600";
      DirectoryMode = "0700";
    };
    Install.WantedBy = [ "sockets.target" ];
  };

  # The bus starts the unit for a notice sent while it's down, rather than
  # running a program of its own: systemd's unit is the only way it runs.
  xdg.dataFile."dbus-1/services/org.freedesktop.Notifications.service".text = ''
    [D-BUS Service]
    Name=org.freedesktop.Notifications
    Exec=${lib.getExe' pkgs.coreutils "false"}
    SystemdService=notices.service
  '';
}
