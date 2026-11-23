# The checks of cadence, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  exampleLaptop,
  holds,
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
  # restarts in fifteen minutes (13 November's slack-unreads, replayed) is
  # told at the tick after its third, once a spell, a failed unit from when it
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
