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
  # The daemon tells an agent from Joel by the node's own names for an
  # agent's units and pi's processes, and reads a unit from a cgroup by
  # the node's own `agents_unit`, word for word, as the pinned mycelium's
  # src/node/asker.rs says them, so what it refuses as Joel's is what the
  # node refuses beside it (ISXEG8, 25 November).
  agents-tells-an-agent-as-the-node-does =
    let
      lines = file: lib.splitString "\n" (builtins.readFile file);
      # What a file's AGENTS_UNITS lists, a name to a line as rustfmt lays
      # them out, or nothing where it lists none.
      units =
        file:
        let
          from = lib.lists.findFirstIndex (lib.hasPrefix "const AGENTS_UNITS") null (lines file);
          rest = lib.drop (from + 1) (lines file);
          until = lib.lists.findFirstIndex (line: line == "];") (lib.length rest) rest;
        in
        if from == null then [ ] else map lib.trim (lib.take until rest);
      # A file's `agents_unit`, from its first line to the brace closing
      # it, or nothing where it has none.
      matching =
        file:
        let
          from = lib.lists.findFirstIndex (lib.hasPrefix "fn agents_unit(") null (lines file);
          rest = lib.drop from (lines file);
          until = lib.lists.findFirstIndex (line: line == "}") (lib.length rest) rest;
        in
        if from == null then [ ] else lib.take (until + 1) rest;
      # The names a file gives pi's process and its host's.
      names =
        file: lib.filter (line: builtins.match "const PI(_HOST)?: &str = .*;" line != null) (lines file);
      ours = ./src/asker.rs;
      nodes = "${self.inputs.mycelium}/src/node/asker.rs";
    in
    holds "agents-tells-an-agent-as-the-node-does"
      "The agents daemon names an agent's units or pi's processes, or reads a unit, otherwise than the node does."
      (
        units ours != [ ]
        && units ours == units nodes
        && lib.length (names ours) == 2
        && names ours == names nodes
        && matching ours != [ ]
        && matching ours == matching nodes
      );
  # The daemon runs in each account of a machine in a realm, following
  # that realm's node and the to-do daemon and reading the launcher's own
  # words, its socket systemd's, and a switch restarts it; a machine in no
  # realm, with no sessions to show, follows no node, but still serves the
  # sessions there are to reopen. It reads what of core runs from
  # update-to-main's answer and cadence's clone, so a try waits for its
  # switch, and the integration's record of switches, so a landing says
  # since when the machine has had it.
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
      alone = lib.concatStringsSep " " (
        lib.toList (account graphical).systemd.user.services.agents.Service.ExecStart
      );
    in
    holds "agents-runs-in-each-account"
      "The agents daemon lacks a source or its socket, outlives a switch, or a realm's node."
      (
        lib.hasInfix "--node %t/mycelium/harbour.sock" started
        && lib.hasInfix "--to-do %t/daemons/to-do.sock" started
        && lib.hasInfix "--windows %t/daemons/windows.sock" started
        && lib.hasInfix "-words.json" started
        && lib.hasInfix "--socket %t/daemons/agents.sock" started
        && lib.hasInfix "/quickshell/by-shell --socket" started
        && lib.hasInfix "--switched /run/update-to-main/last.json --clone " started
        && lib.hasInfix "/cadence/core.git --harness " started
        && lib.hasInfix "/pi-packages/github.com/Jitsusama/agentic-harness.pi --switches " started
        && lib.hasInfix "/integration/switches.jsonl --shells" started
        && lib.elem "PATH=${
          lib.makeBinPath [
            pkgs.git
            pkgs.systemd
          ]
        }" (lib.toList service.Service.Environment)
        && socket.Socket.ListenStream == "%t/daemons/agents.sock"
        && service.Unit.Requires == [ "agents.socket" ]
        && service.Unit.X-SwitchMethod == "restart"
        && lib.hasInfix "--restore " alone
        && !lib.hasInfix "--node" alone
      );
}
