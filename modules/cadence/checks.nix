# The checks of cadence, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
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
  # their records, a figure past its bar or worse than the days before by a
  # fifth and its floor and a crash or failure not seen before are found,
  # too few days before judge nothing, the note keeps to ten lines and
  # says a late finding first, pieces the laptop built for want of the
  # builders are counted apart from the light ones; of the ledger, a
  # finding goes to the longest prefix's owner, is kept once and closes two
  # days after its last, each owner is told once in one message, and a
  # finding past its date is late once; and of the axes, each reading is
  # parsed from the card, the measure, the mesh, the notices and niri, a
  # figure past its bar marks its axis with its owner, a reading nothing
  # measures says so, and an axis is found by the start of its name.
  cadence-asks-when-a-target-slips = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in cadence, its tests included.
  cadence-passes-clippy = passesClippy "cadence-passes-clippy" (pkgs.callPackage ./package.nix { });
}
