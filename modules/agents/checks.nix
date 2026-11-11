# The checks of the agents daemon, which checks.nix gathers with every
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
  # The daemon's tests run as it builds: a session stands as its presence
  # says, in the one order every list keeps, and offers the acts that
  # order gives it, Return doing the harmless one; one not live never
  # looks live; Close says what stays; sessions sit under their
  # orchestrators, never more than two deep, an orchestration standing
  # where its most urgent row would and its waiting members folded; a row
  # says how it stands and what it spent, a tier 2 figure leading past
  # four fifths full; counts and times read as figures do; the drawer's
  # figures say today's only on its day; the sessions together say who
  # needs Joel, who works and the day's dollars; and every word comes from
  # the launcher's own vocabulary.
  agents-makes-the-launchers-rows = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in the agents daemon, its tests included.
  agents-passes-clippy = passesClippy "agents-passes-clippy" (pkgs.callPackage ./package.nix { });
  # The daemon runs in each account of a machine in a realm, following
  # that realm's node and the to-do daemon and reading the launcher's own
  # words, its socket systemd's, and a switch restarts it; a machine in no
  # realm, with no sessions to show, runs none.
  agents-runs-in-each-account =
    let
      home = account (bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        { home-manager.users.someone.jitsusama.mycelium.realm = "harbour"; }
      ]);
      service = home.systemd.user.services.agents;
      socket = home.systemd.user.sockets.agents;
      # Home Manager keeps a unit's ExecStart as a list of its lines.
      started = lib.concatStringsSep " " (lib.toList service.Service.ExecStart);
    in
    holds "agents-runs-in-each-account"
      "The agents daemon lacks a source or its socket, outlives a switch, or runs with no realm."
      (
        lib.hasInfix "--node %t/mycelium/harbour.sock" started
        && lib.hasInfix "--to-do %t/daemons/to-do.sock" started
        && lib.hasInfix "--windows %t/daemons/windows.sock" started
        && lib.hasInfix "-words.json" started
        && lib.hasInfix "--socket %t/daemons/agents.sock" started
        && socket.Socket.ListenStream == "%t/daemons/agents.sock"
        && service.Unit.Requires == [ "agents.socket" ]
        && service.Unit.X-SwitchMethod == "restart"
        && !((account graphical).systemd.user.services ? agents)
      );
}
