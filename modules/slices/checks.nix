# The checks of slices, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
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
{
  # The kernel protects session.slice's, app.slice's and agents.slice's memory only as far as
  # every slice above them is protected, so a floor one of them lacks is a
  # floor nobody has, and nothing says so. session.slice's and app.slice's
  # are hard: the kernel takes a soft one back under pressure, and the
  # compositor reads keys late. agents.slice's is soft, so idle sessions
  # swap before work is ended, but for the hard part the sessions' kitty
  # keeps, which the next check holds; every slice above claims both kinds.
  # Whether the parents' floors cover them is the VM test's to say, since
  # one is a share of memory and another a size.
  session-memory-is-protected =
    let
      inherit (graphical.config) systemd;
      above = kind: [
        systemd.slices.user.sliceConfig.${kind} or null
        systemd.slices."user-".sliceConfig.${kind} or null
        systemd.services."user@".serviceConfig.${kind} or null
      ];
      same = floors: lib.all (floor: floor != null && floor == lib.head floors) floors;
      agents = systemd.user.slices.agents.sliceConfig;
    in
    holds "session-memory-is-protected"
      "A slice above session.slice, app.slice or agents.slice lacks its memory floor, so they have none."
      (
        same (above "MemoryMin")
        && same (above "MemoryLow")
        && systemd.user.slices.session.sliceConfig ? MemoryMin
        && systemd.user.slices.app.sliceConfig ? MemoryMin
        && agents ? MemoryLow
      );

  # The compositor and the shell are how Joel drives the machine, so inside
  # session.slice they come first and keep hard floors of their own. A floor
  # inside one that doesn't cover them protects only part of each, and the
  # portals and the bus take what's left of it, so session.slice's must hold
  # both with a gibibyte to spare for the rest.
  the-desktop-comes-first-in-its-session =
    let
      inherit (graphical.config) systemd home-manager;
      mib =
        size:
        if lib.hasSuffix "G" size then
          lib.toInt (lib.removeSuffix "G" size) * 1024
        else
          lib.toInt (lib.removeSuffix "M" size);
      niri = systemd.user.services.niri.serviceConfig;
      shell = home-manager.users.someone.systemd.user.services.quickshell.Service;
      first = unit: (unit.CPUWeight or 100) == 1000;
    in
    holds "the-desktop-comes-first-in-its-session"
      "niri or the shell isn't first in session.slice, or its floor doesn't cover theirs."
      (
        first niri
        && first shell
        &&
          mib systemd.user.slices.session.sliceConfig.MemoryMin
          >= mib niri.MemoryMin + mib shell.MemoryMin + 1024
      );

  # Agents' sessions sit beside Joel's programs, not among them, where each
  # of their wakeups preempted his, and at a tenth of their weight, where
  # his programs waited on the processor less than a third as long. The
  # sorter watches both, so a session's busy work still goes to jobs.
  sessions-sit-beside-the-programs =
    let
      inherit (graphical.config.systemd.user) slices services;
      weight = slice: slice.sliceConfig.CPUWeight or 100;
      # A slice is watched when it's named alone or with where its terminals'
      # busy work goes, after an `=` the shell quotes.
      watches =
        slice:
        let
          given = "${services.jobs-sorter.serviceConfig.ExecStart} ";
        in
        lib.hasInfix " --under ${slice} " given || lib.hasInfix " --under '${slice}=" given;
    in
    holds "sessions-sit-beside-the-programs"
      "Agents' sessions have no slice beside the apps, the apps don't outweigh it ten times over, or the sorter doesn't watch it."
      (weight slices.app >= 10 * weight slices.agents && watches "app.slice" && watches "agents.slice");

  # A probe waits beside Joel's programs at their weight, so a watcher
  # reading its slice's pressure reads what the rest of the machine makes
  # one wait, and not his programs making each other wait.
  a-probe-waits-beside-the-programs =
    let
      inherit (graphical.config.systemd.user) slices services;
      probe = services.probe.serviceConfig;
    in
    holds "a-probe-waits-beside-the-programs"
      "No probe sits in a slice of its own at the programs' weight, or it isn't the sorter's probe."
      (
        probe.Slice == "probe.slice"
        && slices.probe.sliceConfig.CPUWeight == slices.app.sliceConfig.CPUWeight
        && lib.hasSuffix "/bin/jobs-sorter probe" probe.ExecStart
        && builtins.elem "default.target" services.probe.wantedBy
      );

  # The one kitty every session's window is drawn by comes before the
  # sessions it holds, on the processor and in memory, inside their slice:
  # starved and swapped out beside them, it stopped drawing and lost Joel's
  # keys whenever a session opened (2026-11-09). Its hard floor holds only
  # as far as agents.slice keeps one as large, and the rest of that slice's
  # stays soft.
  the-sessions-kitty-comes-before-its-sessions =
    let
      inherit (graphical.config) home-manager systemd;
      kitty = home-manager.users.someone.systemd.user.services.sessions-kitty.Service;
      agents = systemd.user.slices.agents.sliceConfig;
    in
    holds "the-sessions-kitty-comes-before-its-sessions"
      "The sessions' kitty isn't in agents.slice ahead of its sessions' scopes, or its hard floor isn't one agents.slice keeps."
      (
        kitty.Slice == "agents.slice"
        && (kitty.CPUWeight or 100) > 100
        && kitty ? MemoryMin
        && (agents.MemoryMin or null) == kitty.MemoryMin
      );

  # Work other sessions wait on ranks with them on the disk, where nothing
  # inside the account holds a target, while on the processor it stays below
  # what Joel uses and above idle jobs, and past its ceiling swaps its own
  # memory. A target on one slice there holds back every slice beside it.
  shared-work-ranks-with-the-sessions =
    let
      inherit (graphical.config) systemd environment;
      inherit (systemd.user) slices;
      shared = slices.shared.sliceConfig;
      target = slice: slice.sliceConfig.IODeviceLatencyTargetSec or null;
    in
    holds "shared-work-ranks-with-the-sessions"
      "A slice inside the account holds a disk target, or work other sessions wait on outranks Joel's programs on the processor, has no ceiling, or can't be found."
      (
        lib.all (slice: target slice == null) [
          slices.shared
          slices.agents
          slices.app
          slices.session
        ]
        && shared.CPUWeight < (slices.app.sliceConfig.CPUWeight or 100)
        && shared ? MemoryHigh
        && environment.sessionVariables.SHARED_SLICE == "shared.slice"
      );

  # Work in jobs swaps its own memory past 26% of it, and agents' sessions
  # theirs past 42%, rather than pushing out what an app reaches for next,
  # and both are only ever slowed there: a hard limit would end an
  # evaluation or a session that a slow one lets finish.
  work-swaps-its-own-memory =
    let
      inherit (graphical.config.systemd.user) slices;
      jobs = slices.jobs.sliceConfig;
      agents = slices.agents.sliceConfig;
    in
    holds "work-swaps-its-own-memory"
      "jobs has no ceiling at 26% of memory or agents' sessions none at 42%, so their work pushes apps' memory out to swap, or one has a hard limit that ends work."
      (
        (jobs.MemoryHigh or null) == "26%"
        && !(jobs ? MemoryMax)
        && (agents.MemoryHigh or null) == "42%"
        && !(agents ? MemoryMax)
      );

  # Work yields to what the user works in only while jobs is idle beside
  # app.slice, the sorter runs outside both, pi is told where jobs is, jobs
  # comes back after systemd-oomd ends it, and a window's own scope, where
  # the shell and pi live, isn't idle as well. A dash in the slice's name
  # would nest it under a parent at the default weight, and nothing says so.
  work-yields-to-what-the-person-works-in =
    let
      inherit (graphical.config) systemd environment;
      jobs = systemd.user.services.jobs.serviceConfig;
      sorter = systemd.user.services.jobs-sorter.serviceConfig;
      scope = systemd.user.units."kitty-.scope".text or "";
    in
    holds "work-yields-to-what-the-person-works-in"
      "jobs isn't idle beside app.slice, its sorter runs inside what it sorts, pi can't find it, it stays gone once stopped, or a window's own scope is idle."
      (
        systemd.user.slices.jobs.sliceConfig.CPUWeight == "idle"
        && jobs.Slice == "jobs.slice"
        && jobs.Delegate
        && (jobs.Restart or "no") == "always"
        && sorter.Slice == "session.slice"
        && environment.sessionVariables.JOBS_CGROUP == "jobs.slice/jobs.service"
        && !(lib.hasInfix "idle" scope)
      );

  # Timed work goes ahead of other work only from inside jobs, so it still
  # yields to the apps, at a weight above jobs' own holder, held by a unit
  # that comes back and takes what's written into it, where pi is told.
  # Named any other way, the slice would sit beside the apps instead.
  timed-work-goes-ahead-of-other-work =
    let
      inherit (graphical.config) systemd environment;
      timed = systemd.user.services.timed.serviceConfig;
    in
    holds "timed-work-goes-ahead-of-other-work"
      "Timed work isn't inside jobs above its holder's weight, its holder doesn't take what's written into it or stays gone, or pi can't find it."
      (
        lib.hasPrefix "jobs-" timed.Slice
        && systemd.user.slices.jobs-timed.sliceConfig.CPUWeight > 100
        && timed.Delegate
        && (timed.Restart or "no") == "always"
        && environment.sessionVariables.TIMED_CGROUP == "jobs.slice/${timed.Slice}/timed.service"
      );

  # A terminal's busy work, Joel's own, goes ahead of the work pi starts in
  # jobs.service, only from inside jobs, so it still yields to the apps,
  # and behind timed work. The sorter moves it to a holder that comes back
  # and takes what's written into it. An agent's terminal's goes in with
  # pi's own, and a session's holder stays, since polkit asks Joel for what
  # runs in his slot.
  joels-work-goes-before-agents =
    let
      inherit (graphical.config) systemd;
      awaited = systemd.user.services.awaited.serviceConfig;
      sorter = systemd.user.services.jobs-sorter;
      weight = systemd.user.slices.jobs-awaited.sliceConfig.CPUWeight or 100;
      given =
        flag: value:
        lib.hasInfix " ${flag} ${lib.escapeShellArg value} " "${sorter.serviceConfig.ExecStart} ";
    in
    holds "joels-work-goes-before-agents"
      "A terminal's work isn't moved inside jobs above pi's work and below timed work, or its holder doesn't take what's written into it or stays gone, or an agent's terminal's work or a session's holder goes where Joel's does."
      (
        awaited.Slice == "jobs-awaited.slice"
        && weight > 100
        && weight < systemd.user.slices.jobs-timed.sliceConfig.CPUWeight
        && awaited.Delegate
        && (awaited.Restart or "no") == "always"
        && lib.hasInfix " --jobs jobs.slice/jobs-awaited.slice/awaited.service " sorter.serviceConfig.ExecStart
        && given "--under" "app.slice"
        && given "--under" "agents.slice=jobs.slice/jobs.service"
        && !given "--under" "agents.slice"
        && given "--keep" "mycelium"
        && lib.elem "awaited.service" sorter.requires
      );

  # systemd-oomd ends the whole of the leaf cgroup holding the most swap, so
  # every holder's work is split into a scope for each piece, and a full
  # swap ends the piece that filled it rather than every session's. pi's
  # pieces share one slice at their holder's weight, below a terminal's.
  work-fills-swap-alone =
    let
      inherit (graphical.config.systemd.user) slices;
      sorter = graphical.config.systemd.user.services.jobs-sorter;
      # As the unit quotes it: a value with "=" in it is quoted.
      splits =
        cgroup: lib.hasInfix " --split ${lib.escapeShellArg cgroup} " "${sorter.serviceConfig.ExecStart} ";
    in
    holds "work-fills-swap-alone"
      "A holder's cgroup keeps every command written into it together, so a full swap ends them all at once, or pi's commands each weigh as a terminal's work does."
      (
        splits "jobs.slice/jobs.service=jobs-work.slice"
        && slices.jobs-work.sliceConfig.CPUWeight < slices.jobs-awaited.sliceConfig.CPUWeight
        && slices.jobs-work.sliceConfig.IOWeight < slices.jobs-awaited.sliceConfig.IOWeight
        && splits "jobs.slice/jobs-awaited.slice/awaited.service"
        && splits "jobs.slice/jobs-timed.slice/timed.service"
        && lib.elem graphical.config.systemd.package sorter.path
      );

  # A pi window running a turn nobody typed took the processor from the
  # apps beside builds, so the sorter weighs it low while the harness marks
  # every pi in it as one nobody is at, and pi sessions started outside a
  # terminal are weighed the same way.
  idle-pi-windows-yield =
    let
      sorter = graphical.config.systemd.user.services.jobs-sorter;
      given =
        flag: value:
        lib.hasInfix " ${flag} ${lib.escapeShellArg value} " "${sorter.serviceConfig.ExecStart} ";
    in
    holds "idle-pi-windows-yield"
      "A pi window nobody is at keeps its full weight beside the apps, or one outside a terminal isn't weighed."
      (given "--background" "%t/pi/background" && given "--weigh" "mycelium-pi-");

  # A real-time thread runs ahead of every cgroup's weight, and what agents
  # start inherits the account's real-time limit, so the sorter takes real
  # time from everything in agents' and timers' cgroups. Joel's own terminal
  # work keeps it.
  agents-work-runs-without-real-time =
    let
      sorter = graphical.config.systemd.user.services.jobs-sorter;
      # The word after each --no-real-time, as the unit quotes it.
      taken = map (part: lib.head (lib.splitString " " part)) (
        lib.drop 1 (lib.splitString " --no-real-time " sorter.serviceConfig.ExecStart)
      );
      takes = cgroup: lib.elem (lib.escapeShellArg cgroup) taken;
    in
    holds "agents-work-runs-without-real-time"
      "What agents or timers start may run in real time, ahead of everything Joel uses, or Joel's own terminal work loses it."
      (
        takes "jobs.slice/jobs.service"
        && takes "jobs.slice/jobs-work.slice"
        && takes "jobs.slice/jobs-timed.slice"
        # None holds Joel's, jobs.slice itself included.
        && !lib.any (
          cgroup: cgroup == "jobs.slice" || lib.hasPrefix "jobs.slice/jobs-awaited.slice" cgroup
        ) taken
      );

  # A machine runs behind what the user works in only while libvirt's system
  # machines in machine.slice are weighed as a build is, and inside the
  # account, where a weight buys nothing, rootless containers in its
  # user.slice and session machines in its machine.slice, where their daemon
  # runs, are idle. A machine anywhere else takes an app's weight, and
  # nothing says so.
  machines-yield-to-what-the-person-works-in =
    let
      inherit
        ((bareNixos [
          self.nixosModules.nix-daemon
          self.nixosModules.slices
          self.nixosModules.libvirt
        ]).config
        )
        systemd
        ;
      daemon = systemd.user.services.virtqemud.serviceConfig;
      build = systemd.slices.build.sliceConfig.CPUWeight or 100;
    in
    holds "machines-yield-to-what-the-person-works-in"
      "A machine or container runs at an app's weight: machine.slice isn't weighed as a build is, the account's user.slice or machine.slice isn't idle, or session machines run outside it."
      (
        build < 100
        && (systemd.slices.machine.sliceConfig.CPUWeight or 100) == build
        && (systemd.user.slices.user.sliceConfig.CPUWeight or 100) == "idle"
        && (systemd.user.slices.machine.sliceConfig.CPUWeight or 100) == "idle"
        && (daemon.Slice or "") == "machine.slice"
      );

  # The sorter's tests run as it builds: a busy process group moves after two
  # busy seconds and not one, the shell's group and pi's never move, a busy
  # process inside them moves alone, and a process first seen counts nothing.
  jobs-sorter-sorts-by-behaviour = pkgs.callPackage ./jobs-sorter/package.nix { };

  # systemd-run, in front of a stand-in that says what it was given, adds
  # --slice-inherit for an agent's --user call alone: not Joel's terminal
  # or launcher, nor a call naming a slice, nor one to the system manager.
  an-agents-run-stays-in-its-slice =
    let
      run = pkgs.callPackage ./systemd-run.nix {
        real = "${pkgs.writeShellScript "said" ''echo "$*"''}";
        cgroup = "cgroup";
      };
    in
    pkgs.runCommandLocal "an-agents-run-stays-in-its-slice" { } ''
      ${pkgs.bash}/bin/bash ${./systemd-run-test.sh} ${lib.getExe run}
      touch $out
    '';

  # A workstation's polkit rules, as polkitd assembles them, run in duktape,
  # polkit's own engine, after polkit's init.js: what would ask Joel is
  # refused for a process where agents' work runs, or one gone,
  # and left to ask from his terminal and apps, and a grant stays a grant
  # (agents-ask-test.rules).
  an-agent-raises-no-polkit-prompt =
    let
      # fwupd's rules ask an administrator, so a machine with them shows
      # that asking turned into a refusal.
      machine = bareNixos [
        self.nixosModules.graphical
        self.nixosModules.fwupd
      ];
      rules = machine.config.environment.etc."polkit-1/rules.d/10-nixos.rules".text;
    in
    pkgs.runCommandLocal "an-agent-raises-no-polkit-prompt" { } ''
      ${lib.getExe' pkgs.duktape "duk"} ${./agents-ask-polkit.rules} \
        ${pkgs.polkit.src}/src/polkitbackend/init.js \
        ${pkgs.writeText "10-nixos.rules" rules} ${./agents-ask-test.rules}
      touch $out
    '';
}
// onLinux {
  # clippy finds nothing in the jobs sorter, its tests included.
  jobs-sorter-passes-clippy = passesClippy "jobs-sorter-passes-clippy" (
    pkgs.callPackage ./jobs-sorter/package.nix { }
  );
}
