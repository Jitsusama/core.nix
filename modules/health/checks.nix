# The checks of health, which checks.nix gathers with every other folder's.
{
  exampleLaptop,
  holds,
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# Its tests run on Linux, a Mac's lines read from fixtures, so the
# builders cover both systems' reading.
onLinux {
  # health's tests run as it builds: a journal line is read for its time,
  # priority, the thing that said it (the unit when systemd speaks of
  # one), a crash's program and whose it was by its slice, and a message
  # in bytes; a Mac's unified log line is read alike, its fault as
  # critical, its kernel's as the kernel's and an app's program as a
  # program's; a message's numbers and hex are taken out so its repeats
  # are one signature, and a stack trace is known by its words; a day
  # counts errors, crashes, traces and restarts apart and files each
  # thing under its kind, keeping at most a hundred signatures each; a
  # day is written only once changed, whole, a new one starting empty and
  # a late line of a day done left out; and it's shown a line a kind, the
  # gravest first, with a kind's things or a thing's signatures on asking;
  # and on systemd's own journalctl, each follower reads two boots and
  # follows on across the reboot, and a day whose clock was stepped back
  # is replayed in the order it was written.
  health-tallies-the-days-warnings = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in health, its tests included.
  health-passes-clippy = passesClippy "health-passes-clippy" (pkgs.callPackage ./package.nix { });

  # Health's unit holds a high mark of 256 MiB and no hard limit, so the
  # day's replay at each start, which passes the journals' pages through
  # its cgroup, is slowed there rather than ended; what that bought was
  # measured once, on 19 November, as home.nix says.
  health-gives-back-the-pages-it-reads =
    let
      service = exampleLaptop.config.home-manager.users.joel.systemd.user.services.health.Service;
    in
    holds "health-gives-back-the-pages-it-reads"
      "health has no high mark at 256 MiB, so the day's replay at each start grows the page cache by gigabytes unchecked, or it has a hard limit that ends it mid-replay."
      ((service.MemoryHigh or null) == "256M" && !(service ? MemoryMax));
}
