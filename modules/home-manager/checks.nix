# The checks of home-manager, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  exampleLaptop,
  holds,
  lib,
  onLinux,
  ...
}:
let
  # The example laptop's account with `extra` added to it, so a probe's
  # timers sit beside every timer the laptop's modules write.
  account =
    extra:
    (exampleLaptop.extendModules { modules = [ { home-manager.users.joel = extra; } ]; })
    .config.home-manager.users.joel;
  service = home: name: home.systemd.user.services.${name} or { };
  method = home: name: (service home name).Unit.X-SwitchMethod or null;
  bound = home: name: (service home name).Service.TimeoutStartSec or null;
  true' = "/run/current-system/sw/bin/true";

  # One account holding a probe of each kind of timed service:
  # - bare-probe says nothing of its own, so it takes the defaults;
  # - own-probe says its own switch method and bound, which must win;
  # - unit-probe's timer names the service it runs, unit-probe-run;
  # - probe@one runs an instance of the template probe@.
  probes = account {
    systemd.user.timers.bare-probe.Timer.OnCalendar = "daily";
    systemd.user.services.bare-probe.Service.ExecStart = true';
    systemd.user.timers.own-probe.Timer.OnCalendar = "daily";
    systemd.user.services.own-probe = {
      Unit.X-SwitchMethod = "restart";
      Service = {
        ExecStart = true';
        TimeoutStartSec = "3h";
      };
    };
    systemd.user.timers.unit-probe.Timer = {
      OnCalendar = "daily";
      Unit = "unit-probe-run.service";
    };
    systemd.user.services.unit-probe-run.Service.ExecStart = true';
    systemd.user.timers."probe@one".Timer.OnCalendar = "daily";
    systemd.user.services."probe@".Service.ExecStart = true';
  };
  keeps = {
    "a timed service saying nothing keeps its run over a switch" =
      method probes "bare-probe" == "keep-old";
    "a timed run with no bound of its own ends after an hour" = bound probes "bare-probe" == "1h";
    "a timed service's own switch method wins" = method probes "own-probe" == "restart";
    "a timed service's own bound wins" = bound probes "own-probe" == "3h";
    "a timer's Timer.Unit is the service it keeps" =
      method probes "unit-probe-run" == "keep-old" && !(probes.systemd.user.services ? unit-probe);
    "a template's instance keeps its run on the template" =
      method probes "probe@" == "keep-old" && !(probes.systemd.user.services ? "probe@one");
  };

  # home.nix's assertion as an account says it, or null.
  refusal =
    home:
    lib.findFirst (
      assertion: !assertion.assertion && lib.hasInfix "the switch's setting would hide" assertion.message
    ) null home.assertions;
  # A timer whose service nothing writes, and the same timer with its
  # service named as one the account doesn't write.
  stray = {
    systemd.user.timers.stray-probe.Timer.OnCalendar = "daily";
  };
  strayRefused = refusal (account stray);
  strayExcused = account (stray // { jitsusama.timedRuns.unwritten = [ "stray-probe" ]; });
  writes = {
    "a timer whose service nothing writes is refused, by name" =
      strayRefused != null && lib.hasInfix "stray-probe" strayRefused.message;
    "a service named in jitsusama.timedRuns.unwritten is left out" =
      refusal strayExcused == null && !(strayExcused.systemd.user.services ? stray-probe);
  };
  broken = cases: lib.attrNames (lib.filterAttrs (_: held: !held) cases);
in
onLinux {
  # A timed service keeps its run over a switch and is bounded at an hour,
  # unless it says its own, found by the service its timer names.
  a-switch-leaves-timed-runs-be =
    holds "a-switch-leaves-timed-runs-be"
      "home.nix's switch setting doesn't hold: ${lib.concatStringsSep "; " (broken keeps)}"
      (broken keeps == [ ]);

  # home.nix's assertion refuses a timer whose service the account doesn't
  # write, and lets a machine name such a service.
  every-timer-runs-a-written-service =
    holds "every-timer-runs-a-written-service"
      "home.nix's timers don't hold: ${lib.concatStringsSep "; " (broken writes)}"
      (broken writes == [ ]);
}
