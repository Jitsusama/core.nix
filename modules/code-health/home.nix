# How messy each repository is getting, read each evening: code-health, a
# program of core's own (src/main.rs), reads every repository named in
# `jitsusama.code-health.repos` as its ref holds it in a checkout already on
# the machine, fetching nothing, and writes the day's figures to
# ~/.local/state/code-health/<date>.json: long files and crowded folders
# that don't say why, suppressions, lines repeated from elsewhere, tests
# in quarantine and the points of docs/daemons.md daemons still break.
# It runs before `cadence day`, which holds each figure against the days
# before it, so a repository getting messier is named the day it does.
# jitsusama.code-health.path puts a machine's own programs ahead of core's
# git, for a repository kept by a git of its own that stock git refuses.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.code-health;
  package = pkgs.callPackage ./package.nix { };
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
            type = lib.types.str;
            default = "";
            description = "The repository's folder in a larger one; the whole one when empty.";
          };
        };
      }
    );
    default = { };
    description = "The repositories read each evening, by the name cadence says them by.";
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
          ++ each "--under" (repo: repo.under)
        );
        Environment = [ "PATH=${lib.makeBinPath (cfg.path ++ [ pkgs.git ])}" ];
        Slice = "background.slice";
      };
    };
    systemd.user.timers.code-health = {
      Unit.Description = "Reads how messy each repository is getting, each evening";
      Timer = {
        OnCalendar = "*-*-* 17:40:00";
        Persistent = true;
      };
      Install.WantedBy = [ "timers.target" ];
    };
  };
}
