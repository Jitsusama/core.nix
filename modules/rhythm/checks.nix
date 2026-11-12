# The checks of rhythm, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# It reads swayidle-away's mark and glibc's clock, both Linux's.
onLinux {
  # rhythm's tests run as it builds: a night stops at the last busy minute
  # before four hours' sleep, never at dinner, lunch or a stray key; the
  # usual stop is four weeks' middle, the latest weighing most, so a
  # change of habit shows within a week, and a weekday keeps its own;
  # too few nights guess midnight and say so, unless the four weeks up
  # to a break hold enough; a year of nights is kept past the minutes,
  # and where they span the seasons, tonight leans with the day's light,
  # an hour at most;
  # the logs give only the times Joel typed, each a stop where it's last;
  # Joel reads it plainly; and a walk is suggested only
  # in daylight after a long stretch, apart, three a day and one in poor
  # weather, saying what it's like out.
  rhythm-learns-when-joel-stops = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in rhythm, its tests included.
  rhythm-passes-clippy = passesClippy "rhythm-passes-clippy" (pkgs.callPackage ./package.nix { });
}
