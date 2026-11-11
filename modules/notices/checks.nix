# The checks of the notices daemon, which checks.nix gathers with every
# other folder's; its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  graphical,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  self,
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
  # restart.
  notices-holds-what-the-bus-says = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in the notices daemon, its tests included.
  notices-passes-clippy = passesClippy "notices-passes-clippy" (pkgs.callPackage ./package.nix { });
  # Until the shell's own server goes, nothing starts the daemon: no role
  # imports it, and imported it's wanted by no target, so the
  # notifications name has one server. Imported, it reads the shell's
  # levels and Quiet's file, and its socket is systemd's.
  notices-starts-nowhere-yet =
    let
      home = account (bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        { home-manager.sharedModules = [ self.homeModules.notices ]; }
      ]);
      service = home.systemd.user.services.notices;
      socket = home.systemd.user.sockets.notices;
      # Home Manager keeps a unit's ExecStart as a list of its lines.
      started = lib.concatStringsSep " " (lib.toList service.Service.ExecStart);
    in
    holds "notices-starts-nowhere-yet"
      "The notices daemon is started somewhere, or lacks its levels, Quiet's file or its socket."
      (
        !((account graphical).systemd.user.services ? notices)
        && (service.Install.WantedBy or [ ]) == [ ]
        && (socket.Install.WantedBy or [ ]) == [ ]
        && lib.hasInfix "-notices-levels.json" started
        && lib.hasInfix "--quiet %t/quiet" started
        && lib.hasInfix "--socket %t/daemons/notices.sock" started
        && socket.Socket.ListenStream == "%t/daemons/notices.sock"
        && service.Service.Restart == "always"
      );
}
