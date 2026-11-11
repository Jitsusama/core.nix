# The checks of the notices daemon, which checks.nix gathers with every
# other folder's; its opening comment says what each kind of check proves.
{
  account,
  graphical,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# It serves the shell, which runs only on Linux.
onLinux {
  # The daemon's tests run as it builds, over a session bus of their own:
  # a notice arriving is judged by its sender's level and Quiet, kept
  # newest first to a thousand with none waiting dropped, and told to a
  # watch as one record; an act on it is said on the bus as the
  # specification says, the token first; the name is never taken from
  # another server; and the history, its pictures and its ids outlive a
  # restart. Each notice's rows are its card's, in three places worded
  # each for its list, signed as the desktop knows the sender.
  notices-holds-what-the-bus-says = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in the notices daemon, its tests included.
  notices-passes-clippy = passesClippy "notices-passes-clippy" (pkgs.callPackage ./package.nix { });
  # Each graphical account's session starts the daemon, its socket wanted
  # from the start and the bus starting it for a notice sent while it's
  # down, and it's the notifications name's one server: the shell serves
  # none of its own. It reads the shell's levels and Quiet's file, and the
  # launcher's words and each program's mark, which its rows are said and
  # signed in.
  notices-starts-in-each-account =
    let
      home = account graphical;
      service = home.systemd.user.services.notices;
      socket = home.systemd.user.sockets.notices;
      # Home Manager keeps a unit's ExecStart as a list of its lines.
      started = lib.concatStringsSep " " (lib.toList service.Service.ExecStart);
      named = home.xdg.dataFile."dbus-1/services/org.freedesktop.Notifications.service".text;
      # Whether any of the shell's own QML, as its folder holds it, serves
      # notices. A plain replace, since hasInfix's regex overflows the
      # stack on a file as long as the launcher's.
      serves =
        file:
        let
          text = builtins.readFile file;
        in
        builtins.replaceStrings [ "NotificationServer" ] [ "" ] text != text;
      shellServes = lib.any serves (
        lib.filter (file: lib.hasSuffix ".qml" (toString file)) (
          lib.filesystem.listFilesRecursive ../quickshell
        )
      );
    in
    holds "notices-starts-in-each-account"
      "The notices daemon isn't started in each session, or another serves the notices."
      (
        socket.Install.WantedBy == [ "sockets.target" ]
        && lib.hasInfix "SystemdService=notices.service" named
        && lib.hasInfix "Name=org.freedesktop.Notifications" named
        && lib.hasInfix "-notices-levels.json" started
        && lib.hasInfix "--quiet %t/quiet" started
        && lib.hasInfix "-notices-words.json" started
        && lib.hasInfix "-notices-marks.json" started
        && lib.hasInfix "--socket %t/daemons/notices.sock" started
        && socket.Socket.ListenStream == "%t/daemons/notices.sock"
        && service.Service.Restart == "always"
        && service.Unit.StartLimitIntervalSec == 0
        && !shellServes
      );
}
