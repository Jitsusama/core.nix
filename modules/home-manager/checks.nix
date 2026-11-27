# The checks of home-manager, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  exampleLaptop,
  holds,
  lib,
  onLinux,
  pkgs,
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
  # - unreload-probe says no to a reload and refuses a stop, so the stop
  #   decides; reload-restart-probe says yes to both, and reload wins;
  #   kept-stop-probe refuses a restart but says yes to a stop, and the
  #   refused restart wins;
  # - one-probe says X-ReloadIfChanged = 1;
  # - yes-0 to yes-5 say reload in each of sd-switch's yes spellings,
  #   padded and in capitals, and no-0 to no-5 refuse a stop in each of
  #   its no spellings;
  # - timeout-probe bounds itself by TimeoutSec alone;
  # - misspelt-probe says only a TimeoutStartSec systemd can't read, and
  #   half-probe one it can't read beside a TimeoutSec it can;
  #   kept-bound-probe says a TimeoutStartSec it reads beside a TimeoutSec
  #   it can't;
  # - list-probe, loose-probe and padded-probe say bounds systemd reads
  #   as a list, with no whole number, and padded;
  # - skipped-probe's list ends in a line systemd can't read, so it skips
  #   that line and keeps the one before, and the build says so;
  # - empty-probe's TimeoutStartSec is an empty list, which writes no
  #   line, so it says no bound and none unread;
  # - refused-probe says a span systemd's parse_time refuses, "1.h";
  # - reset-probe's last ExecStart line clears the lines before it;
  # - unit-probe's timer names the service it runs, unit-probe-run;
  # - probe@one runs an instance of the template probe@;
  # - untimed-probe is a oneshot no timer runs;
  # - stray-probe's service nothing writes.
  # sd-switch's to_bool spellings, in 0.6.4's ini.rs, the sd-switch
  # home.nix reads as.
  sdSwitch = "0.6.4";
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
  # One keep per spelling, so a failure names the spelling that broke.
  saysEach =
    kind: spellings: wanted: home:
    lib.listToAttrs (
      lib.imap0 (index: spelling: {
        name = "sd-switch's ${kind} spelling \"${spelling}\", padded and in capitals, reads as ${kind}";
        value = method home "${kind}-${toString index}" == wanted;
      }) spellings
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
        "unreload-probe"
        "reload-restart-probe"
        "kept-stop-probe"
        "one-probe"
        "timeout-probe"
        "misspelt-probe"
        "half-probe"
        "kept-bound-probe"
        "list-probe"
        "skipped-probe"
        "empty-probe"
        "refused-probe"
        "loose-probe"
        "padded-probe"
        "reset-probe"
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
      unreload-probe = oneshot {
        Unit = {
          X-ReloadIfChanged = "off";
          X-StopIfChanged = false;
        };
      };
      reload-restart-probe = oneshot {
        Unit = {
          X-ReloadIfChanged = true;
          X-RestartIfChanged = true;
        };
      };
      kept-stop-probe = oneshot {
        Unit = {
          X-RestartIfChanged = false;
          X-StopIfChanged = "yes";
        };
      };
      one-probe = oneshot { Unit.X-ReloadIfChanged = 1; };
      timeout-probe = oneshot { Service.TimeoutSec = "10min"; };
      misspelt-probe = oneshot { Service.TimeoutStartSec = "10 minuets"; };
      half-probe = oneshot {
        Service = {
          TimeoutStartSec = "10 minuets";
          TimeoutSec = "10min";
        };
      };
      kept-bound-probe = oneshot {
        Service = {
          TimeoutStartSec = "1h";
          TimeoutSec = "half an hour";
        };
      };
      list-probe = oneshot { Service.TimeoutStartSec = [ "1h" ]; };
      skipped-probe = oneshot {
        Service.TimeoutStartSec = [
          "1h"
          "10 minuets"
        ];
      };
      empty-probe = oneshot { Service.TimeoutStartSec = [ ]; };
      refused-probe = oneshot { Service.TimeoutStartSec = "1.h"; };
      loose-probe = oneshot { Service.TimeoutStartSec = ".5h"; };
      padded-probe = oneshot { Service.TimeoutStartSec = "infinity "; };
      reset-probe = oneshot {
        Service.ExecStart = [
          true'
          ""
        ];
      };
      unit-probe-run = oneshot { };
      "probe@" = oneshot { };
      untimed-probe = oneshot { };
    };
  };
  home = account probes;
  # The same account naming stray-probe as one it doesn't write, and
  # bare-probe and reset-probe, which it does: reset-probe's blank last
  # line leaves systemd no ExecStart, but its unit file is still written.
  excused = account (
    probes
    // {
      jitsusama.timedRuns.unwritten = [
        "stray-probe"
        "reset-probe"
        "bare-probe"
      ];
    }
  );

  keeps =
    saysEach "yes" yesSpellings "reload" home // saysEach "no" noSpellings "restart" home // named;
  named = {
    "the sd-switch home.nix reads as is the one nixpkgs ships" =
      exampleLaptop.pkgs.sd-switch.version == sdSwitch;
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
    "a reload said no leaves the refused stop to decide" = method home "unreload-probe" == "restart";
    "a legacy reload key wins over a restart said yes" = method home "reload-restart-probe" == "reload";
    "a refused restart is read as keep-old, ahead of a stop said yes" =
      method home "kept-stop-probe" == "keep-old";
    "a legacy key said as the number 1 is read as yes" = method home "one-probe" == "reload";
    "a TimeoutSec alone bounds the start too" = bound home "timeout-probe" == null;
    "a bound systemd can't read is no bound, so it ends after three hours" =
      bound home "misspelt-probe" == "3h";
    "a readable TimeoutSec bounds the start beside an unreadable TimeoutStartSec" =
      bound home "half-probe" == null;
    "a readable TimeoutStartSec keeps its bound beside an unreadable TimeoutSec" =
      bound home "kept-bound-probe" == null;
    "a bound said as a list, with no whole number, or padded, is the run's own" =
      lib.all (name: bound home name == null)
        [
          "list-probe"
          "skipped-probe"
          "loose-probe"
          "padded-probe"
        ];
    "a span systemd's parse_time refuses is no bound" = bound home "refused-probe" == "3h";
    "a bound systemd can't read is warned of, by name and key" =
      lib.any (lib.hasInfix "misspelt-probe's TimeoutStartSec") home.warnings
      && lib.any (lib.hasInfix "kept-bound-probe's TimeoutSec") home.warnings
      && lib.any (lib.hasInfix "half-probe's TimeoutStartSec") home.warnings
      && !lib.any (lib.hasInfix "loose-probe") home.warnings
      && !lib.any (lib.hasInfix "skipped-probe's TimeoutStartSec [") home.warnings;
    "a list line systemd drops after the one it reads is warned of, by name, key and line" =
      lib.any (lib.hasInfix "skipped-probe's TimeoutStartSec ends in [\"10 minuets\"]") home.warnings;
    "an empty list writes no bound, so it ends after three hours, unwarned" =
      bound home "empty-probe" == "3h" && !lib.any (lib.hasInfix "empty-probe") home.warnings;
    # Only a run left with no bound systemd reads is told it now ends at
    # three hours: one whose other key still bounds it isn't.
    "the warning says three hours only where the base's bound now holds" =
      let
        of = name: lib.filter (lib.hasInfix "${name}'s ") home.warnings;
        hours = name: lib.any (lib.hasInfix "three hours") (of name);
      in
      hours "misspelt-probe"
      && !hours "half-probe"
      && !hours "kept-bound-probe"
      && !hours "skipped-probe";
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
    "a timer whose service's last ExecStart line clears the rest is refused, by name" =
      names hidden home
        "reset-probe";
    "a service named in jitsusama.timedRuns.unwritten is left out" =
      refusal hidden excused == null && !(excused.systemd.user.services ? stray-probe);
    "a written service named in jitsusama.timedRuns.unwritten is refused, by name" =
      names unwritten excused "bare-probe" && !(names unwritten excused "stray-probe");
    "a service whose last ExecStart line clears the rest, named unwritten, is refused" =
      names unwritten excused
        "reset-probe";
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

  # timed.nix reads a timeout's spelling by a copy of systemd's parse_time,
  # so each spelling here is given to the systemd nixpkgs ships, through
  # systemd-analyze timespan, which reads a span as a unit file's timeout
  # is read, and the two must agree on every one.
  systemd-reads-each-span-as-core-does =
    let
      runs = import ./timed.nix { inherit lib; };
      spellings = [
        "1h"
        "10min"
        "2h30min"
        "1h 30min"
        "1h30m"
        ".5h"
        "+1.5h"
        "12.34s.56"
        "1.5"
        "600"
        "0"
        # The micro sign and the Greek mu, each a unit of its own.
        "5000000µs"
        "5000000μs"
        "3 weeks 2days"
        "infinity"
        "10 minuets"
        "half an hour"
        "1.h"
        "3.sec"
        "+.5h"
        "1.5.0h"
        "12.34.56"
        "10+5min"
        "6hrs"
        "30secs"
        "2 mins"
        "-1h"
        # Spans under a microsecond, which systemd counts as zero, and
        # those just over; and a float as home-manager writes 1.0e-7.
        "0.5us"
        "1.5us"
        "0.3μs"
        "0.001ms"
        "0.0001ms"
        "0.000001s"
        "0.0000001s"
        "0.0000000000001y"
        "0.00000000000001y"
        "0min"
        "0.000000"
      ];
      table = lib.concatMapStrings (
        spelling:
        let
          reads = if runs.readsAs spelling then "reads" else "refuses";
          ends = if runs.ends spelling then "ends" else "endless";
        in
        "${reads}\t${ends}\t${spelling}\n"
      ) spellings;
    in
    pkgs.runCommand "systemd-reads-each-span-as-core-does"
      {
        nativeBuildInputs = [ pkgs.systemd ];
        passAsFile = [ "table" ];
        inherit table;
      }
      ''
        wrong=0
        while IFS=$'\t' read -r core ends spelling; do
          # The second line systemd-analyze prints is the span in
          # microseconds; a span of one or more bounds a start.
          if said=$(systemd-analyze timespan -- "$spelling" 2>/dev/null); then
            systemd=reads
            micros=$(printf '%s\n' "$said" | sed -n '2s/.*: *//p')
            case $micros in
              "" | 0 | infinity) lasts=endless ;;
              *) lasts=ends ;;
            esac
          else
            systemd=refuses
            micros=
            lasts=endless
          fi
          if [ "$core" != "$systemd" ]; then
            echo "timed.nix $core \"$spelling\", which systemd $systemd"
            wrong=1
          fi
          if [ "$ends" != "$lasts" ]; then
            echo "timed.nix says \"$spelling\" $ends, which systemd reads as \"$micros\" microseconds"
            wrong=1
          fi
        done < "$tablePath"
        [ "$wrong" = 0 ] && touch "$out"
      '';
}
