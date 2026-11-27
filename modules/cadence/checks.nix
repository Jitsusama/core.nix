# The checks of cadence, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  exampleLaptop,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
let
  # The files the keys check reads: cadence's copies of spend-watch's keys,
  # then spend-watch's own, then the main.rs that hands cadence its runtime
  # folder, in the order reads-the-keys.sh takes them.
  keysRead = builtins.concatStringsSep " " [
    "${./src/hour/measures/unbilled.rs}"
    "${./src/hour/measures/unresolved.rs}"
    "${./src/watch/budgets.rs}"
    "${../spend-watch/src/collection.rs}"
    "${../spend-watch/src/detect.rs}"
    "${../spend-watch/src/standing.rs}"
    "${../spend-watch/src/status.rs}"
    "${../spend-watch/src/config.rs}"
    "${../spend-watch/src/daemon.rs}"
    "${./src/main.rs}"
  ];
  runs = import ../home-manager/timed.nix { inherit lib; };
in
# cadence reads Linux's pressure files and systemd's journal, and builds
# only there.
onLinux {
  # cadence's tests run as it builds: a merge reads its pull request and
  # parents, docs alone are tests only and the kernel or the lock is boot;
  # the follower's look names the core it built over the one it asked at,
  # a switch by hand counts as the follower's do and ends a hold, an ask
  # with no end is under way and a switch put off for a call waits;
  # runs, pressure, memory, a machine-wide kill and the net's round are read
  # from their records; a merge off the machine past twenty minutes, slow
  # VM waits, high pressure, a fresh kill and the net's lag at each doubling
  # are each asked about once; an overdue card is asked for; a question
  # unanswered twenty minutes, or a watcher gone ten, goes to the orchestrator
  # once; a question lands whole and wakes a parked session; the card says
  # the hour's merges, switches and runs; a failed fetch of core is tried
  # again, and a second failure leaves the tick to go on; of the
  # orchestrators and their sessions, one that crashed is revived, one a
  # live pi host holds runs, and one ended or done is left, once per
  # address, not twice in ten minutes nor past three an hour; and of the
  # day, crashes, failed units, kills, pressure and switches are read from
  # their records and each repository's figures from code-health's file, a
  # figure past its bar or worse than the days before by a fifth and its
  # floor, a repository's by any rise past its floor, and a crash or
  # failure not seen before are found, too few days before judge nothing,
  # the note keeps to ten lines and says a late finding first, pieces the
  # laptop built for want of the builders are counted apart from the light
  # ones, the launcher's openings, keys and changes are read from its
  # lines, each key weighed alike, and a day without them has none; of the
  # ledger, a finding goes to the longest prefix's owner, is kept once and
  # closes two days after its last, each owner is told once in one message,
  # and a finding past its date is late once; of the units, a loop of three
  # restarts in fifteen minutes, other than retries on a status the unit
  # names for them (13 November's slack-unreads, replayed), is told at the
  # tick after its third, once a spell, a failed unit from when it
  # failed, a daemon signed out at once and one silent or unreachable once it
  # holds, each with its last error and owner; and of the axes, each reading
  # is parsed from the card, the measure, the mesh, the notices and niri, a
  # figure past its bar marks its axis with its owner, a reading nothing
  # measures says so, and an axis is found by the start of its name.
  cadence-asks-when-a-target-slips = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in cadence, its tests included.
  cadence-passes-clippy = passesClippy "cadence-passes-clippy" (pkgs.callPackage ./package.nix { });
  # The hour's unit holds a high mark of 512 MiB, so the journals and
  # history it reads give their pages back there, no hard limit, whose kill
  # would lose the card, and a timeout, so a throttled hour can't hold back
  # the next; what the mark bought was measured on 19 November, as home.nix
  # says.
  cadence-hour-gives-back-the-pages-it-reads =
    let
      services = exampleLaptop.config.home-manager.users.joel.systemd.user.services;
      service = services.cadence-hour.Service;
    in
    holds "cadence-hour-gives-back-the-pages-it-reads"
      "cadence-hour has no high mark at 512 MiB, so its reads grow the page cache by gigabytes unchecked, or it has a hard limit, whose kill would lose the card, or no timeout, so a throttled hour can hold back the next."
      (
        (service.MemoryHigh or null) == "512M"
        && !(service ? MemoryMax)
        && (service.TimeoutStartSec or null) == "30min"
      );
  # The hour asks nix-store for the closure of the build each service's unit
  # names, so its PATH holds the account's Nix: without it every service
  # would read as one cadence can't judge, and every merge reaching one
  # would wait.
  cadence-hour-finds-nix-store =
    let
      user = exampleLaptop.config.home-manager.users.joel;
      nix = if user.nix.package != null then user.nix.package else pkgs.nix;
      # The path alone, since a check's condition may not name a derivation.
      bin = builtins.unsafeDiscardStringContext "${pkgs.lib.getBin nix}/bin";
      environment = user.systemd.user.services.cadence-hour.Service.Environment or [ ];
    in
    holds "cadence-hour-finds-nix-store"
      "cadence-hour's PATH holds no Nix, so it can't list the closure of the build any service's unit names, and every service reads as unread."
      (
        pkgs.lib.any (
          entry:
          pkgs.lib.hasPrefix "PATH=" entry
          && pkgs.lib.hasInfix bin (builtins.unsafeDiscardStringContext entry)
        ) environment
      );
  # A switch leaves each cadence run an installed timer starts be, the
  # tick, day, review and hour and any added, since waiting on its start
  # would outlast home-manager's patience under load and send the whole
  # switch back; each timer starts the new unit next, so a changed unit
  # reaches the next run, not one under way. With no switch to end one,
  # each is a oneshot, whose start timeout bounds the whole run, and sets
  # one of its own that ends a hang, rather than leaving it to the bound
  # core's base module gives every timed run that sets none, which no
  # cadence run is sized to: zero or infinity ends nothing. A key systemd
  # can't read is refused as such: systemd drops it, so a readable other
  # one then bounds the start, and when both are readable TimeoutStartSec
  # does. A timer's runs and each one's timeout are read by timed.nix, as
  # the base module reads them, so the two never disagree. The four known
  # runs must each be among those found, and no cadence run may be named
  # unwritten, which would leave it unchecked. Stand-in units, each wrong one way, must each be
  # refused, so a check that stopped refusing fails too, and stand-ins
  # right in each way allowed must each pass, so one that refused too
  # much fails as well. It reads the units as declared; that sd-switch
  # keeps an old run it is told to rests on sd-switch.
  cadence-outlasts-a-switch =
    let
      known = [
        "cadence"
        "cadence-day"
        "cadence-review"
        "cadence-hour"
      ];
      cadence = name: name == "cadence" || lib.hasPrefix "cadence-" name;
      problemsOf =
        {
          user,
          unwritten ? [ ],
        }:
        let
          # A target given as one name, not a list, installs it as well.
          installed = lib.filterAttrs (_: t: targets t != [ ]) user.timers;
          targets =
            t:
            lib.concatMap (key: lib.toList (t.Install.${key} or [ ])) [
              "WantedBy"
              "RequiredBy"
            ];
          started = lib.filter cadence (
            runs.timed {
              timers = installed;
              inherit unwritten;
            }
          );
          wrong =
            name:
            let
              service = user.services.${name};
              inherit (runs.timeout service.Service)
                given
                read
                dropped
                value
                ;
              unread = key: builtins.toJSON service.Service.${key};
            in
            if runs.bare user.services name then
              [ "${name} has a timer but no unit" ]
            else
              lib.optional (
                (service.Unit.X-SwitchMethod or null) != "keep-old"
              ) "a switch restarts ${name} and waits on it"
              ++ lib.optional (
                (service.Service.Type or null) != "oneshot"
              ) "${name} isn't a oneshot, so no start timeout ends its hang"
              ++ map (key: "${name}'s ${key} ${unread key} is one systemd can't read") (
                lib.subtractLists read given
              )
              ++ map (
                key:
                "${name}'s ${key} ends in ${builtins.toJSON (runs.late service.Service.${key})}, "
                + "which systemd drops for the line before"
              ) dropped
              ++ lib.optional (read == [ ]) "${name} sets no timeout of its own that systemd reads"
              ++ lib.optional (
                value != null && !runs.ends value
              ) "${name}'s timeout ${builtins.toJSON value} ends no hang";
        in
        lib.concatMap wrong started
        ++ map (name: "${name} is named unwritten, so its run went unchecked") (
          lib.filter cadence unwritten
        )
        ++ map (name: "no installed timer starts ${name}, so it went unchecked") (
          lib.subtractLists started known
        );
      run = timeout: {
        Unit.X-SwitchMethod = "keep-old";
        Service = {
          ExecStart = "cadence";
          Type = "oneshot";
          TimeoutStartSec = timeout;
        };
      };
      # A run as written, its Service section `service` alone.
      unit = service: {
        Unit.X-SwitchMethod = "keep-old";
        Service = {
          ExecStart = "cadence";
        }
        // service;
      };
      timer = {
        Install.WantedBy = [ "timers.target" ];
      };
      timers = lib.genAttrs known (_: timer);
      services = lib.genAttrs known (_: run "1h");
      # Each stand-in, wrong one way, and words its refusal must hold.
      refused = {
        "a timeout of infinity" = {
          user = {
            inherit timers;
            services = services // {
              cadence-day = run "infinity";
            };
          };
          says = "cadence-day's timeout \"infinity\" ends no hang";
        };
        # systemd counts a span down to the microsecond, so these are zero.
        "a timeout under a microsecond" = {
          user = {
            inherit timers;
            services = services // {
              cadence-day = run "0.5us";
            };
          };
          says = "cadence-day's timeout \"0.5us\" ends no hang";
        };
        "a float home-manager writes as zero" = {
          user = {
            inherit timers;
            services = services // {
              cadence-day = run 1.0e-7;
            };
          };
          says = "cadence-day's timeout 1e-07 ends no hang";
        };
        "a timeout systemd can't read" = {
          user = {
            inherit timers;
            services = services // {
              cadence-day = run "10 minuets";
            };
          };
          says = "cadence-day's TimeoutStartSec \"10 minuets\" is one systemd can't read";
        };
        "spans systemd's parse_time refuses" = {
          user = {
            inherit timers;
            services = services // {
              cadence = run "1.h";
              cadence-day = run "+.5h";
              cadence-hour = run "1.5.0h";
              cadence-review = run "10+5min";
            };
          };
          says = "cadence-review's TimeoutStartSec \"10+5min\" is one systemd can't read";
          also = [
            "cadence's TimeoutStartSec \"1.h\" is one systemd can't read"
            "cadence-day's TimeoutStartSec \"+.5h\" is one systemd can't read"
            "cadence-hour's TimeoutStartSec \"1.5.0h\" is one systemd can't read"
          ];
        };
        "a list whose every line systemd can't read" = {
          user = {
            inherit timers;
            services = services // {
              cadence-day = run [
                "10 minuets"
                ""
              ];
            };
          };
          says = "cadence-day's TimeoutStartSec [\"10 minuets\",\"\"] is one systemd can't read";
        };
        "a start timeout systemd can't read over a TimeoutSec it can" = {
          user = {
            inherit timers;
            services = services // {
              cadence-day = lib.recursiveUpdate (run "10 minuets") { Service.TimeoutSec = "30min"; };
            };
          };
          says = "cadence-day's TimeoutStartSec \"10 minuets\" is one systemd can't read";
          # The readable TimeoutSec bounds the start, so nothing hangs.
          unsaid = [
            "hang"
            "sets no timeout"
          ];
        };
        "a TimeoutSec systemd can't read under a start timeout it can" = {
          user = {
            inherit timers;
            services = services // {
              cadence-day = lib.recursiveUpdate (run "1h") { Service.TimeoutSec = "half an hour"; };
            };
          };
          says = "cadence-day's TimeoutSec \"half an hour\" is one systemd can't read";
          unsaid = [
            "hang"
            "sets no timeout"
            "TimeoutStartSec"
          ];
        };
        "an ExecStart of one empty line" = {
          user = {
            inherit timers;
            services = services // {
              cadence = lib.recursiveUpdate (run "1h") { Service.ExecStart = [ "" ]; };
            };
          };
          says = "cadence has a timer but no unit";
        };
        "an ExecStart whose last line clears the lines before it" = {
          user = {
            inherit timers;
            services = services // {
              cadence = lib.recursiveUpdate (run "1h") {
                Service.ExecStart = [
                  "cadence"
                  ""
                ];
              };
            };
          };
          says = "cadence has a timer but no unit";
        };
        "an ExecStart of blanks" = {
          user = {
            inherit timers;
            services = services // {
              cadence = lib.recursiveUpdate (run "1h") { Service.ExecStart = "  "; };
            };
          };
          says = "cadence has a timer but no unit";
        };
        "a timer whose unit has no ExecStart" = {
          user = {
            inherit timers;
            services = services // {
              cadence-day = (run "1h") // {
                Service = lib.removeAttrs (run "1h").Service [ "ExecStart" ];
              };
            };
          };
          says = "cadence-day has a timer but no unit";
        };
        "an empty ExecStart as a null" = {
          user = {
            inherit timers;
            services = services // {
              cadence = lib.recursiveUpdate (run "1h") { Service.ExecStart = null; };
            };
          };
          says = "cadence has a timer but no unit";
        };
        "an empty ExecStart as an empty string" = {
          user = {
            inherit timers;
            services = services // {
              cadence-hour = lib.recursiveUpdate (run "1h") { Service.ExecStart = ""; };
            };
          };
          says = "cadence-hour has a timer but no unit";
        };
        "an instance's timer, its template's run wrong" = {
          user = {
            timers = timers // {
              "cadence-repo@core" = timer;
            };
            services = services // {
              "cadence-repo@" = run "0";
            };
          };
          says = "cadence-repo@'s timeout \"0\" ends no hang";
        };
        "a zero timeout as a number" = {
          user = {
            inherit timers;
            services = services // {
              cadence = run 0;
            };
          };
          says = "cadence's timeout 0 ends no hang";
        };
        "a run with no timeout" = {
          user = {
            inherit timers;
            services = services // {
              cadence-hour = unit { Type = "oneshot"; };
            };
          };
          says = "cadence-hour sets no timeout of its own";
        };
        "a run with no type, so systemd's default" = {
          user = {
            inherit timers;
            services = services // {
              cadence-review = unit { TimeoutStartSec = "1h"; };
            };
          };
          says = "cadence-review isn't a oneshot";
        };
        "a timer not installed" = {
          user = {
            inherit services;
            timers = timers // {
              cadence-review = { };
            };
          };
          says = "no installed timer starts cadence-review";
        };
        "a zero timeout spelled 0min" = {
          user = {
            inherit timers;
            services = services // {
              cadence-day = run "0min";
            };
          };
          says = "cadence-day's timeout \"0min\"";
        };
        "a run a switch restarts" = {
          user = {
            inherit timers;
            services = services // {
              cadence-hour = lib.removeAttrs (run "1h") [ "Unit" ];
            };
          };
          says = "a switch restarts cadence-hour";
        };
        "a run told to restart at a switch" = {
          user = {
            inherit timers;
            services = services // {
              cadence-hour = lib.recursiveUpdate (run "1h") { Unit.X-SwitchMethod = "restart"; };
            };
          };
          says = "a switch restarts cadence-hour";
        };
        "a start timeout that ends nothing over a TimeoutSec that would" = {
          user = {
            inherit timers;
            services = services // {
              cadence-day = lib.recursiveUpdate (run "infinity") { Service.TimeoutSec = "30min"; };
            };
          };
          says = "cadence-day's timeout \"infinity\"";
        };
        "a timer whose unit the account writes empty" = {
          user = {
            timers = timers // {
              cadence-new = timer;
            };
            services = services // {
              cadence-new = lib.recursiveUpdate (run "1h") { Service.ExecStart = [ ]; };
            };
          };
          says = "cadence-new has a timer but no unit";
        };
        "a run that isn't a oneshot" = {
          user = {
            inherit timers;
            services = services // {
              cadence = lib.recursiveUpdate (run "1h") { Service.Type = "simple"; };
            };
          };
          says = "cadence isn't a oneshot";
        };
        "a timer with no unit" = {
          user = {
            inherit services;
            timers = timers // {
              cadence-new = timer;
            };
          };
          says = "cadence-new has a timer but no unit";
        };
        "a list whose last line systemd drops for the one before" = {
          user = {
            inherit timers;
            services = services // {
              cadence = run [
                "1h"
                "10 minuets"
              ];
            };
          };
          says = "cadence's TimeoutStartSec ends in [\"10 minuets\"]";
        };
        # An empty list writes no line, so it is no key given, and not one
        # systemd can't read.
        "a timeout given as an empty list" = {
          user = {
            inherit timers;
            services = services // {
              cadence-day = run [ ];
            };
          };
          says = "cadence-day sets no timeout of its own that systemd reads";
          unsaid = [ "cadence-day's TimeoutStartSec" ];
        };
        "a known run no timer starts" = {
          user = {
            inherit services;
            timers = lib.removeAttrs timers [ "cadence-review" ];
          };
          says = "no installed timer starts cadence-review";
        };
        "a timer named otherwise starting a wrong run" = {
          user = {
            timers = timers // {
              tick.Timer.Unit = "cadence-extra.service";
              tick.Install = timer.Install;
            };
            services = services // {
              cadence-extra = run "0";
            };
          };
          says = "cadence-extra's timeout \"0\"";
        };
      };
      # Each stand-in right in a way the check allows, which it must pass.
      accepted = {
        "units as they should be" = { inherit timers services; };
        "a timeout given as TimeoutSec alone" = {
          inherit timers;
          services = services // {
            cadence-day = unit {
              Type = "oneshot";
              TimeoutSec = "30min";
            };
          };
        };
        "a timeout as a number or in parts" = {
          inherit timers;
          services = services // {
            cadence = run 600;
            cadence-hour = run "1h 30min";
          };
        };
        "timeouts systemd reads in its looser spellings" = {
          inherit timers;
          services = services // {
            cadence = run "12.34s.56";
            cadence-day = run ".5h";
            cadence-hour = run " +1.5h\t";
            cadence-review = run 1.5;
          };
        };
        # The micro sign and the Greek mu are each a unit of their own.
        "timeouts with units side by side, or in micro signs" = {
          inherit timers;
          services = services // {
            cadence = run "1h30m";
            cadence-day = run "5000000µs";
            cadence-hour = run "5000000μs";
          };
        };
        "timeouts of a microsecond, spelt in a larger unit" = {
          inherit timers;
          services = services // {
            cadence = run "0.001ms";
            cadence-day = run 0.000001;
          };
        };
        "a list whose unreadable line comes before the one systemd reads" = {
          inherit timers;
          services = services // {
            cadence = run [
              "10 minuets"
              "1h"
            ];
          };
        };
        "an ExecStart cleared, then given again" = {
          inherit timers;
          services = services // {
            cadence = lib.recursiveUpdate (run "1h") {
              Service.ExecStart = [
                ""
                "cadence"
              ];
            };
          };
        };
        "a timer installed as required" = {
          inherit services;
          timers = timers // {
            cadence-review.Install.RequiredBy = [ "timers.target" ];
          };
        };
        "a timer installed by one target named alone" = {
          inherit services;
          timers = timers // {
            cadence-review.Install.WantedBy = "timers.target";
          };
        };
        "a start timeout that ends a hang over a TimeoutSec that wouldn't" = {
          inherit timers;
          services = services // {
            cadence-day = lib.recursiveUpdate (run "1h") { Service.TimeoutSec = "infinity"; };
          };
        };
        "an instance's timer, its run written as its template" = {
          timers = timers // {
            "cadence-repo@core" = timer;
          };
          services = services // {
            "cadence-repo@" = run "1h";
          };
        };
        "a cadence timer starting what isn't a service" = {
          inherit services;
          timers = timers // {
            cadence-path = timer // {
              Timer.Unit = "cadence-path.path";
            };
          };
        };
        "a cadence unit no timer starts" = {
          inherit timers;
          services = services // {
            cadence-spec = { };
          };
        };
      };
      # A cadence run named unwritten, which the check would no longer see.
      refusedNamed = {
        "a cadence run named unwritten" = {
          user = {
            services = services // {
              cadence-pkg.Service.ExecStart = [ ];
            };
            timers = timers // {
              cadence-pkg = timer;
            };
          };
          unwritten = [ "cadence-pkg" ];
          says = "cadence-pkg is named unwritten, so its run went unchecked";
        };
      };
      # A package's own run named unwritten, which the check must let be.
      acceptedNamed = {
        "a package's run named unwritten" = {
          user = {
            services = services // {
              pkg-run.Service.ExecStart = [ ];
            };
            timers = timers // {
              pkg-run = timer;
            };
          };
          unwritten = [ "pkg-run" ];
        };
      };
      unrefused = lib.attrNames (
        lib.filterAttrs (
          _: case:
          let
            said = problemsOf (
              lib.removeAttrs case [
                "says"
                "also"
                "unsaid"
              ]
            );
          in
          # Each of `also` must be said too, so a case of several stand-ins
          # proves each is refused, not only the first.
          !lib.all (says: lib.any (lib.hasInfix says) said) ([ case.says ] ++ case.also or [ ])
          || lib.any (unsaid: lib.any (lib.hasInfix unsaid) said) (case.unsaid or [ ])
        ) (refused // refusedNamed)
      );
      overrefused = lib.attrNames (
        lib.filterAttrs (_: args: problemsOf args != [ ]) (
          lib.mapAttrs (_: user: { inherit user; }) accepted // acceptedNamed
        )
      );
      home = exampleLaptop.config.home-manager.users.joel;
      problems =
        problemsOf {
          user = home.systemd.user;
          inherit (home.jitsusama.timedRuns) unwritten;
        }
        ++ map (name: "the check no longer refuses ${name}") unrefused
        ++ map (name: "the check refuses ${name}, which it should pass") overrefused;
    in
    holds "cadence-outlasts-a-switch" (lib.concatStringsSep "; " problems) (problems == [ ]);
  # cadence's budget measures match spend-watch's findings by keys copied
  # from it, and its budget alarm reads budgets.json by spend-watch's field
  # names, so one renamed there would read as no budget unbilled, missed or
  # near its allowance rather than as an error: reads-the-keys.sh finds
  # each copy on both sides. It passes only by finding each line, so one
  # whose parser broke would pass while proving nothing:
  # reads-the-keys-catches.sh runs it on the files as they are, then on
  # copies renamed as each guard means, and each must be refused by its
  # own guard. It reads the files alone, so neither package rebuilds for
  # the other.
  cadence-reads-the-keys-spend-watch-writes =
    pkgs.runCommand "cadence-reads-the-keys-spend-watch-writes" { }
      ''
        bash ${./reads-the-keys-catches.sh} ${./reads-the-keys.sh} ${keysRead}
        touch "$out"
      '';
  # cadence-spec's tests run as it builds: a watch of Spec is served the
  # rows a stand-in cadence prints, and one that fails leaves Spec with
  # none and the daemon answering.
  cadence-spec-serves-the-axes = pkgs.callPackage ./spec/package.nix { };
  # clippy finds nothing in cadence-spec, its tests included.
  cadence-spec-passes-clippy = passesClippy "cadence-spec-passes-clippy" (
    pkgs.callPackage ./spec/package.nix { }
  );
}
