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
  # extension stock git doesn't know, and left such a one unread. Core's
  # coreutils come last, for a git of the machine's that is a script: on
  # 13 November one called tr about 60 times a read and found none.
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
      wanted = "PATH=${pkgs.hello}/bin:${profile}:${pkgs.git}/bin:${pkgs.coreutils}/bin";
    in
    holds "code-health-finds-a-machines-programs-first"
      "code-health doesn't find a machine's programs first, or lost core's git or coreutils."
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
  # the day's file is the form cadence reads, and the hour's holds each
  # measure against its target, the hour before and its owner.
  code-health-counts-what-got-messier = pkgs.callPackage ./package.nix { };
  # cadence's hourly read holds each measure against the target and owner
  # a machine sets, so code-health reads each hour and is handed both: a
  # repository's owner, a measure's own owner, and each target.
  code-health-reads-hourly-against-targets =
    let
      home = account (bareNixos [
        {
          home-manager.sharedModules = [
            self.homeModules.code-health
            {
              jitsusama.code-health.repos.some = {
                checkout = "/home/someone/src/some";
                targets.long-files = 3;
                owner = "someone";
                owners.suppressions = "another";
              };
            }
          ];
        }
      ]);
      # home-manager keeps ExecStart as a list of its one command line.
      started = pkgs.lib.concatStringsSep " " (
        pkgs.lib.toList home.systemd.user.services.code-health.Service.ExecStart
      );
      handed = pair: pkgs.lib.hasInfix (pkgs.lib.escapeShellArgs pair) started;
    in
    holds "code-health-reads-hourly-against-targets"
      "code-health doesn't read each hour, or isn't handed a repository's targets and owners."
      (
        home.systemd.user.timers.code-health.Timer.OnCalendar == "hourly"
        && handed [
          "--target"
          "some/long-files=3"
        ]
        && handed [
          "--owner"
          "some=someone"
        ]
        && handed [
          "--owner"
          "some/suppressions=another"
        ]
      );

  # clippy finds nothing in code-health, its tests included.
  code-health-passes-clippy = passesClippy "code-health-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
