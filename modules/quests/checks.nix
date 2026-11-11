# The checks of the quests daemon, which checks.nix gathers with every
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
  # The daemon's tests run as it builds: every quest open and ended is a
  # row in Quests' order, open before ended by priority and rank, then
  # the newest, then A to Z with numbers as numbers; a row says its kind,
  # its priority or the day it ended, and the session holding it, running
  # before parked; a quest in both lists is the open one; the summary says
  # how many are open and driving; and a change tells a watch only the
  # rows that read differently.
  quests-makes-the-launchers-rows = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in the quests daemon, its tests included.
  quests-passes-clippy = passesClippy "quests-passes-clippy" (pkgs.callPackage ./package.nix { });
  # The daemon runs in each account of a machine in a realm, following
  # that realm's node, its socket systemd's, and a switch restarts it; a
  # machine in no realm, with no quests to show, runs none.
  quests-runs-in-each-account =
    let
      home = account (bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        { home-manager.users.someone.jitsusama.mycelium.realm = "harbour"; }
      ]);
      service = home.systemd.user.services.quests;
      socket = home.systemd.user.sockets.quests;
      # Home Manager keeps a unit's ExecStart as a list of its lines.
      started = lib.concatStringsSep " " (lib.toList service.Service.ExecStart);
    in
    holds "quests-runs-in-each-account"
      "The quests daemon lacks its node or its socket, outlives a switch, or runs with no realm."
      (
        lib.hasInfix "--node %t/mycelium/harbour.sock" started
        && lib.hasInfix "--socket %t/daemons/quests.sock" started
        && socket.Socket.ListenStream == "%t/daemons/quests.sock"
        && service.Unit.Requires == [ "quests.socket" ]
        && service.Unit.X-SwitchMethod == "restart"
        && !((account graphical).systemd.user.services ? quests)
      );
}
