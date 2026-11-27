# The checks of the harness's scripts that Linux runs on a schedule or at
# login, which pi's checks.nix gathers beside its own, kept apart since they
# change with default.nix, and the sweep's with what a host's unit clears.
{
  account,
  bareNixos,
  graphical,
  holds,
  lib,
  macWorkstation,
  onLinux,
  pkgs,
  self,
  workstation,
  ...
}:
# They read the Linux workstation's own units, which a Mac can't evaluate.
onLinux {
  # The harness's measure runs each morning in the background, catching up
  # on a morning slept through, with the store's node, and only where the
  # harness is installed; a Mac, with no systemd, gets none.
  pi-measures-each-day-in-the-background =
    let
      home = account workstation;
      mac = account macWorkstation;
      service = home.systemd.user.services.pi-measure;
      timer = home.systemd.user.timers.pi-measure.Timer;
      # home-manager keeps a unit's settings as lists.
      first = value: lib.head (lib.toList value);
    in
    holds "pi-measures-each-day-in-the-background"
      "pi's measure runs outside background.slice, not every day, misses a morning slept through, runs a node off the PATH, fails where the harness isn't installed, or reaches a Mac."
      (
        service.Service.Slice == "background.slice"
        && timer.Persistent
        && lib.hasPrefix "*-*-* " (first timer.OnCalendar)
        && lib.hasPrefix "${lib.getExe pkgs.nodejs} " (first service.Service.ExecStart)
        && lib.hasSuffix "/scripts/measure.ts" (first service.Unit.ConditionPathExists)
        && !(mac.systemd.user.services ? pi-measure)
      );

  # The launcher's list of what the last boot's end took away is written as
  # the account's session starts, with the store's node, only where the
  # harness is installed; a Mac, with no systemd, gets none.
  pi-writes-what-the-last-boot-left =
    let
      home = account workstation;
      mac = account macWorkstation;
      service = home.systemd.user.services.pi-restore-file;
      first = value: lib.head (lib.toList value);
    in
    holds "pi-writes-what-the-last-boot-left"
      "pi's list of the sessions the last boot's end took away isn't written as the session starts, runs outside background.slice or a node off the PATH, logs node's module-type warning, fails where the harness isn't installed, or reaches a Mac."
      (
        lib.elem "default.target" service.Install.WantedBy
        && service.Service.Slice == "background.slice"
        && lib.hasPrefix "${lib.getExe pkgs.nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON " (
          first service.Service.ExecStart
        )
        && lib.hasSuffix "/scripts/restore-file.ts" (first service.Unit.ConditionPathExists)
        && !(mac.systemd.user.services ? pi-restore-file)
      );

  # The windows here joined to another machine's session are noted every
  # five minutes, quietly in the background, only where the harness is installed,
  # so the next boot can offer them back; a Mac gets none.
  pi-notes-windows-on-sessions-elsewhere =
    let
      home = account workstation;
      mac = account macWorkstation;
      service = home.systemd.user.services.pi-windows-elsewhere;
      timer = home.systemd.user.timers.pi-windows-elsewhere;
      first = value: lib.head (lib.toList value);
    in
    holds "pi-notes-windows-on-sessions-elsewhere"
      "pi doesn't note every five minutes the windows joined to sessions other machines hold, runs outside background.slice or a node off the PATH, logs node's module-type warning, fails where the harness isn't installed, or reaches a Mac."
      (
        lib.elem "timers.target" timer.Install.WantedBy
        && first timer.Timer.OnCalendar == "*:0/5"
        && service.Service.Slice == "background.slice"
        && lib.hasPrefix "${lib.getExe pkgs.nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON " (
          first service.Service.ExecStart
        )
        && lib.hasSuffix "/scripts/windows-elsewhere.ts" (first service.Unit.ConditionPathExists)
        && !(mac.systemd.user.timers ? pi-windows-elsewhere)
      );

  # Ended sessions' work trees are swept in a unit of their own, among
  # agents' commands, at the top of each hour, rather than on a pi host's
  # one thread; a Mac, with no systemd, gets none yet. That the
  # hosts stand down after it is the harness's to prove, in its tests.
  pi-sweeps-work-trees-in-a-unit-of-its-own =
    let
      home = account workstation;
      mac = account macWorkstation;
      service = home.systemd.user.services.pi-sweep;
      timer = home.systemd.user.timers.pi-sweep;
      first = value: lib.head (lib.toList value);
      script = first service.Unit.ConditionPathExists;
      git = lib.makeBinPath [ home.programs.git.package ];
      profile = "${home.home.profileDirectory}/bin";
      # The slices it runs in, whose hard limit would kill it as its own
      # would: a graphical machine declares them (modules/slices, which the
      # workstation alone doesn't import), and an account could too, in
      # home-manager's own shape, on either machine.
      noHardLimit =
        name:
        !((graphical.config.systemd.user.slices.${name}.sliceConfig or { }) ? MemoryMax)
        && lib.all (one: !((one.systemd.user.slices.${name}.Slice or { }) ? MemoryMax)) [
          home
          (account graphical)
        ];
      # And what holds the account's manager, whose hard limit would kill
      # it as surely: the account's slices and the manager's own unit, as
      # a template or for any one uid, on either machine, since the
      # workstation's memory module sets the manager's unit too.
      noHardLimitAbove =
        lib.all
          (
            machine:
            let
              inherit (machine.config) systemd;
              named = p: all: lib.attrValues (lib.filterAttrs (n: _: lib.hasPrefix p n) all);
              slices = map (one: one.sliceConfig or { }) (named "user-" systemd.slices);
              managers = map (one: one.serviceConfig or { }) (named "user@" systemd.services);
            in
            lib.all (unit: !(unit ? MemoryMax)) (
              [ (systemd.slices.user.sliceConfig or { }) ] ++ slices ++ managers
            )
          )
          [
            workstation
            graphical
          ];
      # A machine giving provider modules, a git of its own and a realm whose
      # host shows what a host's unit clears and whose folders the sweep's
      # PATH takes, written as systemd needs whatever they hold. Its builder
      # key runs the agents' SSH proxy, which the bare machine lacks.
      given = account (bareNixos [
        self.nixosModules.workstation
        {
          home-manager.users.someone.programs.git.package = pkgs.gitMinimal;
          home-manager.users.someone.jitsusama.pi = {
            agentProxy.builderKey = "/keys/store_ed25519";
            sweep.providerModules = [
              "/one/providers.ts"
              "/two/providers.ts"
            ];
            hosts.one = {
              path = [
                "/tools/bin"
                "/100% sure/bin"
                ''/odd"dir\bin''
              ];
              pi = "${pkgs.emptyDirectory}/pi";
              node = "${pkgs.emptyDirectory}/node";
              proof = builtins.toFile "pi-host-proof.json" (
                builtins.toJSON {
                  kind = "pi-host-runtime-proof";
                  version = 1;
                  harness = "0000000000000000000000000000000000000000";
                  files = 14;
                  cases = 71;
                  provenAt = "2026-11-10T11:00:00.000Z";
                  runtime = builtins.unsafeDiscardStringContext "${pkgs.emptyDirectory}";
                }
              );
            };
          };
        }
      ]);
      givenSweep = given.systemd.user.services.pi-sweep.Service;
      givenHost = given.systemd.user.services.pi-host-one-background.Service;
      # The agents' SSH proxy, which the sweep gets where its realm's host
      # does, and where it doesn't, no agent at all. Whether one runs is
      # read from the proxy's own socket unit, not from the rule that
      # decides it.
      proxy = ''"SSH_AUTH_SOCK=%t/pi-agent.sock"'';
      proxied = account: account.systemd.user.sockets ? pi-agent-proxy;
      givenProfile = "${given.home.profileDirectory}/bin";
      cleared = unit: lib.sort lib.lessThan (lib.toList unit.UnsetEnvironment);
      # What bash reads as it starts the room check, beyond what node reads.
      bashReads = [
        "BASHOPTS"
        "BASH_ENV"
        "PS4"
        "SHELLOPTS"
      ];
    in
    holds "pi-sweeps-work-trees-in-a-unit-of-its-own"
      "pi's tree sweep doesn't run only at the top of each hour by the wall clock, catching up an hour missed while off, runs outside jobs-work.slice or a node off the PATH, isn't one run to its end, runs a script other than the one it checks for, logs node's module-type warning, fails where the harness isn't installed, is stopped by a switch or by its own clock before two hours, starts without waiting for the machine to have room, fails when that wait skips the hour, has disk weight other than 1, has its state folder's disk reads capped other than at 40 MB/s, waits for room past 15 minutes or without reading when the last sweep ended, isn't made to give back memory past 6 GiB, or has a hard limit, its own, its slices' or its account manager's, to kill it at, has a PATH other than the account's git, its hosts' folders and the account's own, names provider modules other than a machine gave it, has an SSH agent other than the agents' proxy its hosts get, leaves a variable that loads code that a pi host's unit clears, or a variable bash reads as it starts, or reaches a Mac."
      (
        lib.elem "timers.target" timer.Install.WantedBy
        &&
          timer.Timer == {
            OnCalendar = "hourly";
            Persistent = true;
          }
        && service.Service.Type == "oneshot"
        && service.Service.Slice == "jobs-work.slice"
        && service.Service.TimeoutStartSec == "2h"
        && service.Service.MemoryHigh == "6G"
        && !(service.Service ? ExecStartPre)
        && lib.hasPrefix "/nix/store/" (first service.Service.ExecCondition)
        && lib.hasSuffix (
          "/bin/pi-sweep-room /proc/pressure /proc/meminfo 900 30"
          + " %S/pi/agentic-harness.pi/work/sweep.lock.outside"
        ) (first service.Service.ExecCondition)
        && service.Service.IOWeight == 1
        && (service.Service.IOReadBandwidthMax or null) == "${home.xdg.stateHome} 40M"
        && !(service.Service ? MemoryMax)
        && noHardLimit "jobs"
        && noHardLimit "jobs-work"
        && noHardLimitAbove
        && service.Unit.X-SwitchMethod == "keep-old"
        && lib.hasSuffix "/scripts/pi-sweep.ts" script
        &&
          first service.Service.ExecStart
          == "${lib.getExe pkgs.nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON ${script}"
        &&
          lib.toList service.Service.Environment == [
            ''"PATH=${git}:${profile}:/run/current-system/sw/bin"''
          ]
        && !(proxied home)
        &&
          cleared service.Service
          == lib.sort lib.lessThan (cleared givenHost ++ bashReads ++ [ "SSH_AUTH_SOCK" ])
        &&
          lib.toList givenSweep.Environment == [
            ''"PATH=${pkgs.gitMinimal}/bin:/tools/bin:/100%% sure/bin:/odd\"dir\\bin:${givenProfile}:/run/current-system/sw/bin"''
            ''"PI_SWEEP_PROVIDERS=/one/providers.ts:/two/providers.ts"''
            proxy
          ]
        && proxied given
        && lib.elem proxy (lib.toList givenHost.Environment)
        && cleared givenSweep == lib.sort lib.lessThan (cleared givenHost ++ bashReads)
        && !(mac.systemd.user.timers ? pi-sweep)
        && !(mac.systemd.user.services ? pi-sweep)
      );

  # The sweep's room check, the one its unit runs, over readings of a
  # quiet machine, one at each mark, one past each, a wait run out, a
  # stall that clears, a last sweep too long ago to wait for, none or one
  # ahead of the clock, a stamp aging in the last sleep, a wait that fails
  # itself, and readings it can't make or that aren't numbers.
  pi-sweep-waits-for-room =
    let
      service = (account workstation).systemd.user.services.pi-sweep;
      waits = lib.head (lib.toList service.Service.ExecCondition);
      program = lib.head (lib.splitString " " waits);
    in
    pkgs.runCommandLocal "pi-sweep-waits-for-room" { } ''
      ${lib.getExe pkgs.bash} ${./sweep-room-waits.sh} ${program}
      touch $out
    '';
}
