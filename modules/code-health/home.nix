# How messy each repository is getting, read each hour: code-health, a
# program of core's own (src/main.rs), reads every repository named in
# `jitsusama.code-health.repos` as its ref holds it in a checkout already on
# the machine, fetching nothing, and writes the day's figures to
# ~/.local/state/code-health/<date>.json: long files and crowded folders
# that don't say why, suppressions, lines repeated from elsewhere, tests
# in quarantine and the points of docs/daemons.md daemons still break;
# lint rules turned off, files nothing reaches, folders with no note and
# VM test lines; and Terraform not as tofu fmt writes it, what tflint
# finds, what trivy finds insecure, what tofu validate refuses, what
# says nothing of what it is and what no tofu test tests.
# It writes hour.json beside them, each figure against the target and
# owner a repository's `targets`, `owner` and `owners` give it, and the
# figure the hour before, which `cadence hour` reads at ten past. A read
# takes about half a minute, so hourly costs little. It runs before
# `cadence day`, which holds each figure against the days before it, so a
# repository getting messier is named the day it does.
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
  judged = lib.concatLists (
    lib.mapAttrsToList (
      name: repo:
      byMeasure "--target" name repo.targets
      ++ lib.optionals (repo.owner != null) [
        "--owner"
        "${name}=${repo.owner}"
      ]
      ++ byMeasure "--owner" name repo.owners
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
        };
      }
    );
    default = { };
    description = "The repositories read each hour, by the name cadence says them by.";
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
