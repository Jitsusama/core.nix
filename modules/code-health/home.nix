# How messy each repository is getting, read each hour: code-health, a
# program of core's own (src/main.rs), reads every repository named in
# `jitsusama.code-health.repos` as its ref holds it in a checkout already on
# the machine, fetching nothing, and writes the day's figures to
# ~/.local/state/code-health/<date>.json: long files and crowded folders
# that don't say why, suppressions, lines repeated from elsewhere, tests
# in quarantine and the points of docs/daemons.md daemons still break;
# lint rules turned off, files nothing reaches, folders with no note and
# VM test lines; and Terraform not as tofu fmt writes it, what tflint
# finds held to all its own rules and Google's, what trivy finds insecure, what tofu validate refuses or warns of, what
# says nothing of what it is and what no tofu test tests.
# It writes hour.json beside them, each figure against the target and
# owner a repository's `targets`, `owner` and `owners` give it, and the
# figure the hour before, with the first files counted, those of a file
# holding more than the day before's counts by file first, and each of
# those files' entries by the owner `folders` gives its folder, but for
# the files code-health couldn't read, which go to the measure's owner;
# `cadence hour` reads it at ten past. A read takes about half a minute,
# so hourly costs little. It runs before `cadence day`, which holds each
# figure against the days before it, so a repository getting messier is
# named the day it does.
# jitsusama.code-health.path puts a machine's own programs ahead of core's
# git, for a repository kept by a git of its own that stock git refuses;
# core's coreutils come after, for such a git that is a script.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.code-health;
  package = pkgs.callPackage ./package.nix { };
  # A target or owner as its flag takes it, NAME/MEASURE=VALUE.
  byMeasure =
    flag: name: values:
    lib.concatLists (
      lib.mapAttrsToList (measure: value: [
        flag
        "${name}/${measure}=${toString value}"
      ]) values
    );
  # The measure code-health counts the files its own reader couldn't read
  # under, read from hour.rs itself, so a rename there can't leave the
  # keeper's flag naming a measure code-health no longer has, which would
  # refuse every hourly read.
  unreadComments = lib.pipe (builtins.readFile ./src/hour.rs) [
    (lib.splitString "\n")
    (map (builtins.match ''pub const UNREAD_COMMENTS: &str = "([^"]+)";''))
    (lib.findFirst (named: named != null) (throw "hour.rs names no UNREAD_COMMENTS"))
    builtins.head
  ];
  # The files code-health's own reader couldn't read are its keeper's to
  # mend, wherever they lie, unless a repository names another for them.
  kept = lib.optionalAttrs (cfg.owner != null) { ${unreadComments} = cfg.owner; };
  judged = lib.concatLists (
    lib.mapAttrsToList (
      name: repo:
      byMeasure "--target" name repo.targets
      ++ lib.optionals (repo.owner != null) [
        "--owner"
        "${name}=${repo.owner}"
      ]
      ++ byMeasure "--owner" name (kept // repo.owners)
      ++ lib.concatLists (
        lib.mapAttrsToList (folder: owner: [
          "--owner"
          "${name}:${folder}=${owner}"
        ]) repo.folders
      )
    ) cfg.repos
  );
  each =
    flag: value:
    lib.concatLists (
      lib.mapAttrsToList (name: repo: [
        flag
        "${name}=${value repo}"
      ]) cfg.repos
    );
in
{
  options.jitsusama.code-health.repos = lib.mkOption {
    type = lib.types.attrsOf (
      lib.types.submodule {
        options = {
          checkout = lib.mkOption {
            type = lib.types.str;
            description = "A checkout of the repository, or its git folder.";
          };
          ref = lib.mkOption {
            type = lib.types.str;
            default = "origin/main";
            description = "The ref read, so what's measured is what was merged.";
          };
          under = lib.mkOption {
            type = lib.types.coercedTo lib.types.str lib.toList (lib.types.listOf lib.types.str);
            default = [ ];
            example = [
              "apps/web"
              "infra/web"
            ];
            description = "The repository's folders in a larger one, each path said from its top when there are several; the whole one when none.";
          };
          targets = lib.mkOption {
            type = lib.types.attrsOf lib.types.number;
            default = { };
            example = {
              long-files = 0;
            };
            description = "The most each measure should be, by its name; a measure left out is unjudged.";
          };
          owner = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
            description = "Who is told when one of the repository's measures is past its target.";
          };
          owners = lib.mkOption {
            type = lib.types.attrsOf lib.types.str;
            default = { };
            description = "Who owns a measure, by its name, ahead of the repository's owner.";
          };
          folders = lib.mkOption {
            type = lib.types.attrsOf lib.types.str;
            default = { };
            example = {
              "modules/net" = "someone";
            };
            description = "Who owns what the hour names in a folder, by its path from the repository's top, the deepest first, ahead of the measure's owner; never the files code-health couldn't read.";
          };
        };
      }
    );
    default = { };
    description = "The repositories read each hour, by the name cadence says them by.";
  };

  options.jitsusama.code-health.owner = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    default = null;
    description = ''
      Who keeps code-health itself: told, in every repository, of the files
      whose comments its stale reader couldn't read, which only its own
      misreading leaves, ahead of the repository's owner, unless the
      repository's owners name another for unread-comments.
    '';
  };

  options.jitsusama.code-health.path = lib.mkOption {
    type = lib.types.listOf (lib.types.either lib.types.package lib.types.str);
    default = [ ];
    description = ''
      Programs code-health finds first on its PATH, ahead of the git core
      gives it, such as a git that reads a repository stock git refuses:
      packages, or the folders of profiles kept outside the store, each of
      whose bin is searched.
    '';
  };

  config = lib.mkIf (cfg.repos != { }) {
    home.packages = [ package ];

    systemd.user.services.code-health = {
      Unit = {
        Description = "Reads how messy each repository is getting";
        # A laptop asleep at both times starts both on waking; the day is
        # read once its figures are written.
        Before = [ "cadence-day.service" ];
      };
      Service = {
        Type = "oneshot";
        ExecStart = lib.escapeShellArgs (
          [ (lib.getExe package) ]
          ++ each "--repo" (repo: repo.checkout)
          ++ each "--ref" (repo: repo.ref)
          ++ lib.concatLists (
            lib.mapAttrsToList (
              name: repo:
              lib.concatMap (folder: [
                "--under"
                "${name}=${folder}"
              ]) repo.under
            ) cfg.repos
          )
          ++ judged
        );
        Environment = [
          "PATH=${
            lib.makeBinPath (
              cfg.path
              ++ [
                pkgs.git
                pkgs.coreutils
              ]
            )
          }"
        ];
        Slice = "background.slice";
      };
    };
    systemd.user.timers.code-health = {
      Unit.Description = "Reads how messy each repository is getting, each hour";
      Timer = {
        # On the hour, so the hour's file is fresh when `cadence hour` reads
        # it at ten past, and the 17:00 read is in before cadence's day.
        OnCalendar = "hourly";
        Persistent = true;
      };
      Install.WantedBy = [ "timers.target" ];
    };
  };
}
