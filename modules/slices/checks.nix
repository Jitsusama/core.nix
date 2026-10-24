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
        && lib.hasSuffix " --jobs jobs.slice/jobs-awaited.slice/awaited.service" sorter.serviceConfig.ExecStart
        && lib.elem "awaited.service" sorter.requires
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
