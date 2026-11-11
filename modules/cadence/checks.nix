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
  # unanswered twenty minutes, or a watcher gone ten, goes to the steward
  # once; a question lands whole and wakes a parked session; the card says
  # the hour's merges, switches and runs; and of the orchestrators and
  # their sessions, one that crashed is revived, one a live pi host holds
  # runs, and one ended or done is left, once per address, not twice in
  # ten minutes nor past three an hour.
  cadence-asks-when-a-target-slips = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in cadence, its tests included.
  cadence-passes-clippy = passesClippy "cadence-passes-clippy" (pkgs.callPackage ./package.nix { });
}
