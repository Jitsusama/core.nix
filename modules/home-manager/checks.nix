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
  startBound = home: name: (service home name).Service.TimeoutStartSec or null;
  runBound = home: name: (service home name).Service.RuntimeMaxSec or null;
  true' = "/run/current-system/sw/bin/true";
  daily.Timer.OnCalendar = "daily";

  # One account holding a probe of each kind of timed service. Its
  # assertions are read, never enforced, so the refused probes sit beside
  # the rest:
  # - bare-probe says nothing of its own, so it takes the defaults;
  # - own-probe says its own switch method and bounds, which must win;
  # - unit-probe's timer names the service it runs, unit-probe-run;
  # - probe@one runs an instance of the template probe@;
  # - stray-probe's service nothing writes;
  # - restart-probe restarts at a switch by the legacy key alone;
  # - timeout-probe bounds itself by TimeoutSec alone.
  probes = {
    systemd.user.timers = {
      bare-probe = daily;
      own-probe = daily;
      unit-probe.Timer = daily.Timer // {
        Unit = "unit-probe-run.service";
      };
      "probe@one" = daily;
      stray-probe = daily;
      restart-probe = daily;
      timeout-probe = daily;
    };
    systemd.user.services = {
      bare-probe.Service.ExecStart = true';
      own-probe = {
        Unit.X-SwitchMethod = "restart";
        Service = {
          ExecStart = true';
          TimeoutStartSec = "3h";
          RuntimeMaxSec = "5h";
        };
      };
      unit-probe-run.Service.ExecStart = true';
      "probe@".Service.ExecStart = true';
      restart-probe = {
        Unit.X-RestartIfChanged = true;
        Service.ExecStart = true';
      };
      timeout-probe.Service = {
        ExecStart = true';
        TimeoutSec = "10min";
      };
    };
  };
  home = account probes;
  # The same account naming stray-probe as one it doesn't write, and
  # bare-probe, which it does.
  excused = account (
    probes
    // {
      jitsusama.timedRuns.unwritten = [
        "stray-probe"
        "bare-probe"
      ];
    }
  );

  keeps = {
    "a timed service saying nothing keeps its run over a switch" =
      method home "bare-probe" == "keep-old";
    "a timed oneshot's run with no bound of its own ends after an hour" =
      startBound home "bare-probe" == "1h";
    "a timed run of another type with no bound of its own ends after three hours" =
      runBound home "bare-probe" == "3h";
    "a timed service's own switch method wins" = method home "own-probe" == "restart";
    "a timed service's own bounds win" =
      startBound home "own-probe" == "3h" && runBound home "own-probe" == "5h";
    "a timer's Timer.Unit is the service it keeps" =
      method home "unit-probe-run" == "keep-old" && !(home.systemd.user.services ? unit-probe);
    "a template's instance keeps its run on the template" =
      method home "probe@" == "keep-old" && !(home.systemd.user.services ? "probe@one");
  };

  # The message of home.nix's assertion holding `words`, as an account
  # says it, or null.
  refusal =
    words: home':
    let
      found = lib.findFirst (
        assertion: !assertion.assertion && lib.hasInfix words assertion.message
      ) null home'.assertions;
    in
    if found == null then null else found.message;
  names =
    words: home': name:
    lib.hasInfix name (toString (refusal words home'));
  hidden = "the switch's setting would hide";
  overruled = "the timed-run defaults would overrule";
  unwritten = "names services the account writes";
  writes = {
    "a timer whose service nothing writes is refused, by name" = names hidden home "stray-probe";
    "a service named in jitsusama.timedRuns.unwritten is left out" =
      refusal hidden excused == null && !(excused.systemd.user.services ? stray-probe);
    "a written service named in jitsusama.timedRuns.unwritten is refused, by name" =
      names unwritten excused "bare-probe" && !(names unwritten excused "stray-probe");
    "a legacy restart under the default keep-old is refused, by name" =
      names overruled home
        "restart-probe";
    "a TimeoutSec under the default hour is refused, by name" = names overruled home "timeout-probe";
    "a service saying its own switch method and bounds is not refused" =
      !(names overruled home "own-probe") && !(names overruled home "bare-probe");
  };
  broken = cases: lib.attrNames (lib.filterAttrs (_: held: !held) cases);
in
onLinux {
  # A timed service keeps its run over a switch and is bounded, a oneshot
  # at an hour and another at three, unless it says its own, found by the
  # service its timer names.
  a-switch-leaves-timed-runs-be =
    holds "a-switch-leaves-timed-runs-be"
      "home.nix's switch setting doesn't hold: ${lib.concatStringsSep "; " (broken keeps)}"
      (broken keeps == [ ]);

  # home.nix's assertions refuse a timer whose service the account doesn't
  # write, a service the defaults would quietly overrule, and a written
  # service named as unwritten; a machine may name an unwritten one.
  every-timer-runs-a-written-service =
    holds "every-timer-runs-a-written-service"
      "home.nix's timers don't hold: ${lib.concatStringsSep "; " (broken writes)}"
      (broken writes == [ ]);
}
