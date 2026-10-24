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
  # The kernel protects session.slice's and app.slice's memory only as far as
  # every slice above them is protected, so a floor one of them lacks is a
  # floor nobody has, and nothing says so. Only a hard floor counts: the
  # kernel takes a soft one back under pressure, and the compositor reads keys
  # late. Whether the parents' floor covers both is the VM test's to say,
  # since one is a share of memory and the other a size.
  session-memory-is-protected =
    let
      inherit (graphical.config) systemd;
      parents = [
        systemd.slices.user.sliceConfig.MemoryMin or null
        systemd.slices."user-".sliceConfig.MemoryMin or null
        systemd.services."user@".serviceConfig.MemoryMin or null
      ];
      floors = parents ++ [
        systemd.user.slices.session.sliceConfig.MemoryMin or null
        systemd.user.slices.app.sliceConfig.MemoryMin or null
      ];
    in
    holds "session-memory-is-protected"
      "A slice above session.slice or app.slice lacks its hard memory floor, so they have none."
      (lib.all (floor: floor != null) floors && lib.all (floor: floor == lib.head parents) parents);

  # Work in jobs swaps its own memory past half of it, rather than pushing
  # out what an app reaches for next, and is only ever slowed there: a hard
  # limit would end an evaluation that a slow one lets finish.
  work-swaps-its-own-memory =
    let
      jobs = graphical.config.systemd.user.slices.jobs.sliceConfig;
    in
    holds "work-swaps-its-own-memory"
      "jobs has no ceiling at half of memory, so its work pushes apps' memory out to swap, or it has a hard limit that ends work."
      ((jobs.MemoryHigh or null) == "50%" && !(jobs ? MemoryMax));

  # Work yields to what the user works in only while jobs is idle beside
  # app.slice, the sorter runs outside both, pi is told where jobs is, jobs
  # comes back after systemd-oomd ends it, and a window's own scope, where
  # the shell and pi live, isn't idle as well. A dash in the slice's name
  # would nest it under a parent at the default weight, and nothing says so.
  work-yields-to-what-the-user-works-in =
    let
      inherit (graphical.config) systemd environment;
      jobs = systemd.user.services.jobs.serviceConfig;
      sorter = systemd.user.services.jobs-sorter.serviceConfig;
      scope = systemd.user.units."kitty-.scope".text or "";
    in
    holds "work-yields-to-what-the-user-works-in"
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
  # and takes what's written into it.
  joels-work-goes-before-agents =
    let
      inherit (graphical.config) systemd;
      awaited = systemd.user.services.awaited.serviceConfig;
      sorter = systemd.user.services.jobs-sorter;
      weight = systemd.user.slices.jobs-awaited.sliceConfig.CPUWeight or 100;
    in
    holds "joels-work-goes-before-agents"
      "A terminal's work isn't moved inside jobs above pi's work and below timed work, or its holder doesn't take what's written into it or stays gone."
      (
        awaited.Slice == "jobs-awaited.slice"
        && weight > 100
        && weight < systemd.user.slices.jobs-timed.sliceConfig.CPUWeight
        && awaited.Delegate
        && (awaited.Restart or "no") == "always"
        && lib.hasInfix " --jobs jobs.slice/jobs-awaited.slice/awaited.service " sorter.serviceConfig.ExecStart
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
  machines-yield-to-what-the-user-works-in =
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
    holds "machines-yield-to-what-the-user-works-in"
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
}
// onLinux {
  # clippy finds nothing in the jobs sorter, its tests included.
  jobs-sorter-passes-clippy = passesClippy "jobs-sorter-passes-clippy" (
    pkgs.callPackage ./jobs-sorter/package.nix { }
  );
}
