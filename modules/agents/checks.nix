# The checks of the agents daemon, which checks.nix gathers with every
# other folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# It serves the shell, which runs only on Linux.
onLinux {
  # The daemon's tests run as it builds: a session stands as its presence
  # says, in the one order every list keeps, and offers the acts that
  # order gives it, Return doing the harmless one; one not live never
  # looks live; Close says what stays; sessions sit under their
  # orchestrators, never more than two deep, an orchestration standing
  # where its most urgent row would and its waiting members folded; a row
  # says how it stands and what it spent, a tier 2 figure leading past
  # four fifths full; counts and times read as figures do; the drawer's
  # figures say today's only on its day; the sessions together say who
  # needs Joel, who works and the day's dollars; and every word comes from
  # the launcher's own vocabulary.
  agents-makes-the-launchers-rows = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in the agents daemon, its tests included.
  agents-passes-clippy = passesClippy "agents-passes-clippy" (pkgs.callPackage ./package.nix { });
}
