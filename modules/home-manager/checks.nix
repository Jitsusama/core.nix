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
  # The bound core's drop-in gives a service, null when it writes none,
  # and "none" when it writes one saying no TimeoutStartSec, so a drop-in
  # bounding a run some other way never passes for no drop-in.
  bound =
    home: name:
    let
      dropIn = home.xdg.configFile."systemd/user/${name}.service.d/timed-run.conf" or null;
      found = builtins.match ".*TimeoutStartSec=([^\n]*).*" dropIn.text;
    in
    if dropIn == null then
      null
    else if found == null then
      "none"
    else
      lib.head found;
  true' = "/run/current-system/sw/bin/true";
  daily.Timer.OnCalendar = "daily";
  oneshot = extra: {
    Service = {
      Type = "oneshot";
      ExecStart = true';
    }
    // extra.Service or { };
    Unit = extra.Unit or { };
  };

  # One account holding a probe of each kind of service. Its assertions
  # are read, never enforced, so the refused probe sits beside the rest:
  # - bare-probe, a timed oneshot, says nothing of its own;
  # - exec-probe is timed but not a oneshot;
  # - own-probe says its own switch method and bound, which must win;
  # - restart-probe and reload-probe say a legacy key, one as a string;
  # - both-probe says reload, padded and in capitals, over a restart
  #   refused, and reload wins;
  # - kept-probe refuses both a restart and a stop, and the refused
  #   restart wins; stop-probe refuses only a stop;
  # - mixed-probe refuses a stop but says yes to a restart, and the
  #   refused stop wins; halt-probe says yes only to a stop;
  # - one-probe says X-ReloadIfChanged = 1;
  # - yes-0 to yes-5 say reload in each of sd-switch's yes spellings,
  #   padded and in capitals, and no-0 to no-5 refuse a stop in each of
  #   its no spellings;
  # - timeout-probe bounds itself by TimeoutSec alone;
  # - unit-probe's timer names the service it runs, unit-probe-run;
  # - probe@one runs an instance of the template probe@;
  # - untimed-probe is a oneshot no timer runs;
  # - stray-probe's service nothing writes.
  # sd-switch's to_bool spellings, at 308f45c's ini.rs.
  yesSpellings = [
    "1"
    "yes"
    "y"
    "true"
    "t"
    "on"
  ];
  noSpellings = [
    "0"
    "no"
    "n"
    "false"
    "f"
    "off"
  ];
  # One probe per spelling, named kind-index, each saying `key` as it.
  spellingProbes =
    kind: key: spellings:
    lib.listToAttrs (
      lib.imap0 (index: spelling: {
        name = "${kind}-${toString index}";
        value = oneshot { Unit.${key} = " ${lib.toUpper spelling} "; };
      }) spellings
    );
  spelledProbes =
    spellingProbes "yes" "X-ReloadIfChanged" yesSpellings
    // spellingProbes "no" "X-StopIfChanged" noSpellings;
  saysEach =
    kind: spellings: wanted: home:
    lib.all (index: method home "${kind}-${toString index}" == wanted) (
      lib.range 0 (lib.length spellings - 1)
    );

  probes = {
    systemd.user.timers =
      lib.mapAttrs (_: _: daily) spelledProbes
      // lib.genAttrs [
        "bare-probe"
        "exec-probe"
        "own-probe"
        "restart-probe"
        "reload-probe"
        "both-probe"
        "kept-probe"
        "stop-probe"
        "mixed-probe"
        "halt-probe"
        "one-probe"
        "timeout-probe"
        "probe@one"
        "stray-probe"
      ] (_: daily)
      // {
        unit-probe.Timer = daily.Timer // {
          Unit = "unit-probe-run.service";
        };
      };
    systemd.user.services = spelledProbes // {
      bare-probe = oneshot { };
      exec-probe.Service = {
        Type = "exec";
        ExecStart = true';
      };
      own-probe = oneshot {
        Unit.X-SwitchMethod = "stop-start";
        Service.TimeoutStartSec = "5h";
      };
      restart-probe = oneshot { Unit.X-RestartIfChanged = true; };
      reload-probe = oneshot { Unit.X-ReloadIfChanged = "yes"; };
      both-probe = oneshot {
        Unit = {
          X-ReloadIfChanged = " Y ";
          X-RestartIfChanged = false;
        };
      };
      kept-probe = oneshot {
        Unit = {
          X-RestartIfChanged = "no";
          X-StopIfChanged = false;
        };
      };
      stop-probe = oneshot { Unit.X-StopIfChanged = false; };
      mixed-probe = oneshot {
        Unit = {
          X-RestartIfChanged = true;
          X-StopIfChanged = "off";
        };
      };
      halt-probe = oneshot { Unit.X-StopIfChanged = "T"; };
      one-probe = oneshot { Unit.X-ReloadIfChanged = 1; };
      timeout-probe = oneshot { Service.TimeoutSec = "10min"; };
      unit-probe-run = oneshot { };
      "probe@" = oneshot { };
      untimed-probe = oneshot { };
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
    "a timed oneshot saying nothing keeps its run over a switch" =
      method home "bare-probe" == "keep-old";
    "a timed oneshot with no bound of its own ends after three hours" = bound home "bare-probe" == "3h";
    "a timed service of another type restarts as before, unbounded" =
      method home "exec-probe" == null && bound home "exec-probe" == null;
    "a timed oneshot's own switch method and bound win" =
      method home "own-probe" == "stop-start"
      && bound home "own-probe" == null
      && (service home "own-probe").Service.TimeoutStartSec == "5h";
    "a legacy restart key alone is read as sd-switch's stop-start" =
      method home "restart-probe" == "stop-start";
    "a legacy reload key, said as a string, is read as reload" = method home "reload-probe" == "reload";
    "a legacy reload key wins over a refused restart, padded and in any case" =
      method home "both-probe" == "reload";
    "a refused restart is read as keep-old, ahead of a refused stop" =
      method home "kept-probe" == "keep-old";
    "a legacy stop key saying no is read as restart" = method home "stop-probe" == "restart";
    "a refused stop is read as restart, ahead of a restart said yes" =
      method home "mixed-probe" == "restart";
    "a legacy stop key alone is read as sd-switch's stop-start" =
      method home "halt-probe" == "stop-start";
    "each of sd-switch's yes spellings, padded and in capitals, reads as yes" =
      saysEach "yes" yesSpellings "reload"
        home;
    "each of sd-switch's no spellings, padded and in capitals, reads as no" =
      saysEach "no" noSpellings "restart"
        home;
    "a legacy key said as the number 1 is read as yes" = method home "one-probe" == "reload";
    "a TimeoutSec alone bounds the start too" = bound home "timeout-probe" == null;
    "a timer's Timer.Unit is the service it keeps" =
      method home "unit-probe-run" == "keep-old" && !(home.systemd.user.services ? unit-probe);
    "a template's instance keeps its run, and its bound, on the template" =
      method home "probe@" == "keep-old"
      && bound home "probe@" == "3h"
      && !(home.systemd.user.services ? "probe@one");
    "a oneshot no timer runs is left alone" =
      method home "untimed-probe" == null && bound home "untimed-probe" == null;
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
  unwritten = "names services the account writes";
  writes = {
    "a timer whose service nothing writes is refused, by name" = names hidden home "stray-probe";
    "a service named in jitsusama.timedRuns.unwritten is left out" =
      refusal hidden excused == null && !(excused.systemd.user.services ? stray-probe);
    "a written service named in jitsusama.timedRuns.unwritten is refused, by name" =
      names unwritten excused "bare-probe" && !(names unwritten excused "stray-probe");
  };
  broken = cases: lib.attrNames (lib.filterAttrs (_: held: !held) cases);
in
onLinux {
  # A timed oneshot keeps its run over a switch and is bounded at three
  # hours, unless it says its own; another type, or a service no timer
  # runs, is left to the switch as before.
  a-switch-leaves-timed-runs-be =
    holds "a-switch-leaves-timed-runs-be"
      "home.nix's switch setting doesn't hold: ${lib.concatStringsSep "; " (broken keeps)}"
      (broken keeps == [ ]);

  # home.nix's assertions refuse a timer whose service the account doesn't
  # write, and a written service named as unwritten; a machine may name an
  # unwritten one.
  every-timer-runs-a-written-service =
    holds "every-timer-runs-a-written-service"
      "home.nix's timers don't hold: ${lib.concatStringsSep "; " (broken writes)}"
      (broken writes == [ ]);
}
