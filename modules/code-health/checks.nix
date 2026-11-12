# The checks of code-health, which checks.nix gathers with every other
# folder's.
{
  account,
  bareNixos,
  holds,
  passesClippy,
  pkgs,
  self,
  ...
}:
{
  # A machine's own programs come first on code-health's PATH, with core's
  # git after: stock git refuses a repository its own git keeps with an
  # extension stock git doesn't know, and left such a one unread.
  code-health-finds-a-machines-programs-first =
    let
      # A package, and a profile kept outside the store, whose bin is searched.
      first = [
        pkgs.hello
        "/home/someone/.local/state/tools/profile"
      ];
      home = account (bareNixos [
        {
          home-manager.sharedModules = [
            self.homeModules.code-health
            {
              jitsusama.code-health = {
                path = first;
                repos.some.checkout = "/home/someone/src/some";
              };
            }
          ];
        }
      ]);
      # Written out rather than built as the module builds it.
      profile = "/home/someone/.local/state/tools/profile/bin";
      wanted = "PATH=${pkgs.hello}/bin:${profile}:${pkgs.git}/bin";
    in
    holds "code-health-finds-a-machines-programs-first"
      "code-health doesn't find a machine's programs first, or lost core's git."
      (home.systemd.user.services.code-health.Service.Environment == [ wanted ]);

  # code-health's tests run as it builds: a long file or crowded folder
  # counts only without its reason, a reason must open a sentence, a
  # suppression is read in its own language's way and not in a string, a
  # run of lines repeated from elsewhere is counted once where it repeats
  # and braces alone repeat nothing, each quarantined test is counted by
  # its name and a repository keeping no quarantine has no figure for it,
  # each point a daemon waits on is counted by its daemon and a repository
  # holding no daemons to docs/daemons.md has no figure for it, git's batch
  # answer is split by its sizes, a repository's ref and folder go to it,
  # and the day's file is the form cadence reads.
  code-health-counts-what-got-messier = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in code-health, its tests included.
  code-health-passes-clippy = passesClippy "code-health-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
