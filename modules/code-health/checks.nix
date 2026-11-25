# The checks of code-health, which checks.nix gathers with every other
# folder's.
{
  account,
  bareNixos,
  holds,
  onLinux,
  passesClippy,
  pkgs,
  self,
  ...
}:
{
  # code-health's tests run as it builds: a long file or crowded folder
  # counts only without its reason, a reason must open a sentence, a
  # suppression is read in its own language's way and not in a string, a
  # run of lines repeated from elsewhere is counted once where it repeats
  # and braces alone repeat nothing, each quarantined test is counted by
  # its name and a repository keeping no quarantine has no figure for it,
  # each point a daemon waits on is counted by its daemon and a repository
  # holding no daemons to docs/daemons.md has no figure for it, git's batch
  # answer is split by its sizes, a repository's ref and folder go to it, a
  # folder on the disk is read as git's tree would hold it and what the
  # stale-paths measure finds there is held to a waiting list by file and
  # path, the day's file is the form cadence reads, and the hour's holds each
  # measure against its target, the hour before and its owner. As the
  # quality standard has them, it names each lint rule a config turns off,
  # each file nothing reaches, each folder that says nothing of what it
  # holds and each VM test's lines; and of its Terraform, each file not as
  # tofu fmt writes it, each finding of tflint's, held to all its own
  # rules and Google's, a workspace it can't load among them, each of trivy's security findings, with the real
  # tools, each error and each warning tofu validate finds, each variable or output with
  # no description, and each workspace with no README or no tofu test.
  code-health-counts-what-got-messier = pkgs.callPackage ./package.nix { };
  # code-health and core's every-long-file-says-why name the same planted
  # folders crowded, each one's note giving its reason in another way or
  # failing to (crowds-as-core-does.sh), so a folder that rule passes is
  # never counted crowded in the hourly figures, nor one it fails let by.
  code-health-crowds-as-core-does =
    pkgs.runCommandLocal "code-health-crowds-as-core-does"
      {
        nativeBuildInputs = [
          (pkgs.callPackage ./package.nix { })
          pkgs.git
          pkgs.python3
        ];
      }
      ''
        HOME=$TMPDIR bash ${./crowds-as-core-does.sh} \
          ${../../tests/rules/every-long-file-says-why.py}
        touch $out
      '';
  # A repository can be several folders of a larger one, such as one
  # team's Terraform kept in a few places of a monorepo; each is handed
  # to code-health, so none goes unread.
  code-health-reads-each-folder-it-is-given =
    let
      home = account (bareNixos [
        {
          home-manager.sharedModules = [
            self.homeModules.code-health
            {
              jitsusama.code-health.repos = {
                some = {
                  checkout = "/home/someone/src/mono";
                  under = [
                    "apps/web"
                    "infra/web"
                  ];
                };
                one = {
                  checkout = "/home/someone/src/mono";
                  under = "apps/one";
                };
              };
            }
          ];
        }
      ]);
      started = pkgs.lib.concatStringsSep " " (
        pkgs.lib.toList home.systemd.user.services.code-health.Service.ExecStart
      );
      handed =
        folder:
        pkgs.lib.hasInfix (pkgs.lib.escapeShellArgs [
          "--under"
          folder
        ]) started;
    in
    holds "code-health-reads-each-folder-it-is-given"
      "code-health isn't handed every folder a repository is given."
      (handed "some=apps/web" && handed "some=infra/web" && handed "one=apps/one");
  # cadence's hourly read holds each measure against the target and owner
  # a machine sets, so code-health reads each hour and is handed both: a
  # repository's owner, a measure's own owner, and each target; and what
  # its own reader couldn't read goes to who keeps code-health, unless a
  # repository names another for it.
  code-health-reads-hourly-against-targets =
    let
      home = account (bareNixos [
        {
          home-manager.sharedModules = [
            self.homeModules.code-health
            {
              jitsusama.code-health.owner = "keeper";
              jitsusama.code-health.repos.some = {
                checkout = "/home/someone/src/some";
                targets.long-files = 3;
                owner = "someone";
                owners.suppressions = "another";
              };
              jitsusama.code-health.repos.one = {
                checkout = "/home/someone/src/one";
                owners.unread-comments = "another";
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
        && handed [
          "--owner"
          "some/unread-comments=keeper"
        ]
        && handed [
          "--owner"
          "one/unread-comments=another"
        ]
        && !handed [
          "--owner"
          "one/unread-comments=keeper"
        ]
      );

  # With no keeper named, code-health is handed no owner for what its own
  # reader couldn't read, so those files go to each repository's owner;
  # and the measure the module routes is the one code-health counts them
  # under, so renaming it in either alone is caught.
  code-health-routes-unread-comments-as-it-counts-them =
    let
      home = account (bareNixos [
        {
          home-manager.sharedModules = [
            self.homeModules.code-health
            {
              jitsusama.code-health.repos.some = {
                checkout = "/home/someone/src/some";
                owner = "someone";
              };
            }
          ];
        }
      ]);
      started = pkgs.lib.concatStringsSep " " (
        pkgs.lib.toList home.systemd.user.services.code-health.Service.ExecStart
      );
      counted = ''pub const UNREAD_COMMENTS: &str = "unread-comments";'';
      routed = "{ unread-comments = cfg.owner; }";
    in
    holds "code-health-routes-unread-comments-as-it-counts-them"
      "code-health is handed an owner for unread files with no keeper, or routes another name than it counts."
      (
        !pkgs.lib.hasInfix "unread-comments" started
        && pkgs.lib.hasInfix counted (builtins.readFile ./src/hour.rs)
        && pkgs.lib.hasInfix routed (builtins.readFile ./home.nix)
      );

  # clippy finds nothing in code-health, its tests included.
  code-health-passes-clippy = passesClippy "code-health-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
# What follows runs the Linux workstation's own programs, which a Mac can't run.
// onLinux {
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
}
