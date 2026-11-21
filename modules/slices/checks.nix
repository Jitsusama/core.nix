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

  # Under every core's load, the processor's one power budget holds each
  # near the same clock, so one of Joel's threads ran no faster on a
  # performance core than on an efficient one. A machine that names its
  # efficient cores keeps agents' work and the builds to them, which leaves
  # the performance cores and their budget to what he uses, while his own
  # terminal work, timed work and the daemons he asks stay free. The
  # account's manager is handed the cpuset controller, without which its
  # units can't be held to any cores. A machine naming none holds nothing,
  # and the laptop's hardware names its cores but holds nothing without
  # the slices.
  agents-work-keeps-off-the-performance-cores =
    let
      machine = modules: (bareNixos modules).config;
      slices =
        named:
        machine [
          self.nixosModules.graphical
          { jitsusama.slices.efficientCores = named; }
        ];
      held = slices "4-15";
      free = slices null;
      laptop = machine [ self.nixosModules.dell-xps-14-da14260 ];
      cores = config: [
        config.systemd.user.services.jobs.serviceConfig.AllowedCPUs or null
        config.systemd.user.slices.jobs-work.sliceConfig.AllowedCPUs or null
        config.systemd.slices.build.sliceConfig.AllowedCPUs or null
      ];
      # jobs.slice itself among them, since a pin there would hold every
      # slice under it.
      joels = config: [
        config.systemd.user.slices.jobs.sliceConfig.AllowedCPUs or null
        config.systemd.user.slices.jobs-awaited.sliceConfig.AllowedCPUs or null
        config.systemd.user.slices.jobs-timed.sliceConfig.AllowedCPUs or null
        config.systemd.user.slices.background.sliceConfig.AllowedCPUs or null
      ];
      delegate = config: config.systemd.services."user@".serviceConfig.Delegate or "";
      delegated =
        config: lib.isString (delegate config) && lib.elem "cpuset" (lib.splitString " " (delegate config));
      none = [
        null
        null
        null
      ];
      nothing = none ++ [ null ];
    in
    holds "agents-work-keeps-off-the-performance-cores"
      "Naming the efficient cores didn't hold agents' work and builds to them alone, naming none held them anyway, or the laptop's hardware held them without the slices."
      (
        cores held == [
          "4-15"
          "4-15"
          "4-15"
        ]
        && joels held == nothing
        && delegated held
        && cores free == none
        && !delegated free
        && laptop.jitsusama.slices.efficientCores == "4-15"
        && cores laptop == none
        && !delegated laptop
      );

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

  # The account's daemons share eight processors' time at most, declared,
  # since a quota set only at runtime is lost at the next boot, as one was
  # on 20 November, and nothing says so.
  background-work-keeps-to-eight-processors =
    holds "background-work-keeps-to-eight-processors"
      "background.slice declares no quota of eight processors' time, so the account's daemons can take every idle core after a boot."
      ((graphical.config.systemd.user.slices.background.sliceConfig.CPUQuota or null) == "800%");

  # A woken drawing thread of Joel's Chrome or Slack runs sooner under load
  # when it asks for a short slice, so the sorter has them ask, in his programs'
  # slice only: an agent's asking would run sooner beside his.
  joels-programs-ask-short-slices =
    let
      sorter = graphical.config.systemd.user.services.jobs-sorter.serviceConfig;
      # The word after each of a flag's appearances, as the unit quotes it.
      after =
        flag:
        map (part: lib.head (lib.splitString " " part)) (
          lib.drop 1 (lib.splitString " ${flag} " sorter.ExecStart)
        );
    in
    holds "joels-programs-ask-short-slices"
      "Joel's Chrome or Slack don't ask for a short slice, so their drawing threads wait longer under load, or another slice's programs ask too."
      (
        after "--quick" == [ (lib.escapeShellArg "app.slice") ]
        &&
          lib.sort lib.lessThan (after "--quick-name") == [
            "chrome"
            "slack"
          ]
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

  # A service Joel's own traffic passes through, as a tunnel's daemon does,
  # keeps its memory out of swap and under a hard floor, and comes first for
  # the disk, so agents' load can't leave it waiting on its own pages. The
  # kernel keeps a floor only as far as every slice above claims one, so
  # system.slice claims the carriers' floors too. A name no service has is
  # refused, since it would make an empty unit rather than protect one.
  a-carrier-keeps-its-memory =
    let
      machine =
        carriers: extra:
        (bareNixos [
          self.nixosModules.nix-daemon
          self.nixosModules.slices
          { jitsusama.slices.carriers = carriers; }
          extra
        ]).config;
      inherit (machine [ "tunnel" ] { systemd.services.tunnel.serviceConfig.ExecStart = "/bin/true"; })
        systemd
        ;
      tunnel = systemd.services.tunnel.serviceConfig;
      refused = lib.any (claim: !claim.assertion) (machine [ "none-such" ] { }).assertions;
      # WARP's daemon is one wherever it runs, unless the machine says otherwise.
      warp =
        (bareNixos [
          self.nixosModules.nix-daemon
          self.nixosModules.slices
          { services.cloudflare-warp.enable = true; }
        ]).config.jitsusama.slices.carriers;
    in
    holds "a-carrier-keeps-its-memory"
      "A carrier of Joel's traffic can swap, has no hard floor or one system.slice doesn't claim, isn't first for the disk, or a name no service has was taken."
      (
        (tunnel.MemorySwapMax or null) == 0
        && (tunnel.MemoryMin or null) == "256M"
        && (tunnel.IOWeight or null) == 1000
        && (systemd.slices.system.sliceConfig.MemoryMin or null) == "256M"
        && refused
        && warp == [ "cloudflare-warp" ]
      );

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

  # efficient-cores, on a stand-in sysfs, holds only the efficient cores'
  # ceiling, within each one's own range, frees it to each one's full clock,
  # waits behind another's lock, refuses words it doesn't take, and holds
  # nothing where there are none. util-linux gives the test flock, to hold
  # the lock itself.
  efficient-cores-hold-only-theirs =
    pkgs.runCommandLocal "efficient-cores-hold-only-theirs" { nativeBuildInputs = [ pkgs.util-linux ]; }
      ''
        mkdir work && cd work
        ${pkgs.bash}/bin/bash ${./efficient-cores-test.sh} ${
          lib.getExe (pkgs.callPackage ./efficient-cores.nix { })
        }
        touch $out
      '';

  # A workstation's polkit rules, as polkitd assembles them, run in duktape,
  # polkit's own engine, after polkit's init.js: what would ask Joel is
  # refused for a process where agents' work runs, or one gone, but an
  # agent's run0 or pkexec, which asks him once for its command, and a root
  # step Joel named in jitsusama.slices.agentsMayRepeat, which is granted;
  # it's left to ask from his terminal and apps, and a grant stays a grant
  # (agents-ask-test.rules). The cases run on a machine naming no root
  # step, as every machine is by default, and on one naming one. A name
  # that isn't one unit's whole name, or is one run0 makes, is refused.
  an-agent-raises-no-polkit-prompt =
    let
      # fwupd's rules ask an administrator, so a machine with them shows
      # that asking turned into a refusal.
      machineNaming =
        named:
        (bareNixos [
          self.nixosModules.graphical
          self.nixosModules.fwupd
          { jitsusama.slices.agentsMayRepeat = named; }
        ]).config;
      rulesOf = named: (machineNaming named).environment.etc."polkit-1/rules.d/10-nixos.rules".text;
      refuses =
        unit: lib.any (claim: !claim.assertion) (machineNaming { ${unit} = [ "restart" ]; }).assertions;
      namesHeld =
        lib.all refuses [
          "run-u42.service"
          "nix-daemon"
          "*.service"
        ]
        && !refuses "nix-daemon.service";
      run = catalogued: named: ''
        ${lib.getExe' pkgs.duktape "duk"} ${./agents-ask-polkit.rules} \
          ${pkgs.polkit.src}/src/polkitbackend/init.js \
          ${pkgs.writeText "10-nixos.rules" (rulesOf named)} \
          ${pkgs.writeText "catalogued.js" "var catalogued = ${lib.boolToString catalogued};"} \
          ${./agents-ask-test.rules}
      '';
    in
    assert lib.assertMsg namesHeld "agentsMayRepeat took a name that isn't one system unit's";
    pkgs.runCommandLocal "an-agent-raises-no-polkit-prompt" { } ''
      ${run false { }}
      ${run true { "nix-daemon.service" = [ "restart" ]; }}
      touch $out
    '';
}
# What follows needs Linux to build or to run, so a Mac leaves it out.
// onLinux {
  # The sorter's tests run as it builds: a busy process group moves after two
  # busy seconds and not one, the shell's group and pi's never move, a busy
  # process inside them moves alone, and a process first seen counts nothing.
  jobs-sorter-sorts-by-behaviour = pkgs.callPackage ./jobs-sorter/package.nix { };

  # clippy finds nothing in the jobs sorter, its tests included.
  jobs-sorter-passes-clippy = passesClippy "jobs-sorter-passes-clippy" (
    pkgs.callPackage ./jobs-sorter/package.nix { }
  );
}
