# repos lists every repository on the machine once, with every checkout of
# it and every tree cut from each (src/lib.rs says how). It reads git's own
# files as it runs, and merges in what GitHub last held, which a timer asks
# every five minutes through the GitHub broker (src/github.rs says why).
# Where it looks differs between machines: a machine whose trees lie in a
# folder of their own, under no checkout, adds that folder. So do the few
# repositories the App can't see that a machine has read through gh's
# sign-in instead (src/gh.rs says how narrowly). A second timer counts how
# far each head is from the branch it follows, with git, and keeps the
# counts for every reading to merge in too (src/upstreams.rs says why).
{
  config,
  lib,
  pkgs,
  ...
}:
let
  repos = pkgs.callPackage ./package.nix { };
  inherit (config.jitsusama.repos) roots throughGh;
  # The command's settings, as defaults a caller may still set for one
  # run. gh is named only where a machine lists repositories for it, so
  # it's never asked elsewhere.
  settings = {
    REPOS_ROOTS = lib.concatStringsSep ":" roots;
    REPOS_GIT = lib.getExe config.programs.git.package;
  }
  // lib.optionalAttrs (throughGh != [ ]) {
    REPOS_THROUGH_GH = lib.concatStringsSep " " throughGh;
    REPOS_GH = lib.getExe config.programs.gh.package;
  };
  wrapped = pkgs.symlinkJoin {
    name = "repos-${repos.version}";
    paths = [ repos ];
    nativeBuildInputs = [ pkgs.makeWrapper ];
    postBuild = ''
      wrapProgram $out/bin/repos ${
        lib.concatStringsSep " " (
          lib.mapAttrsToList (name: value: "--set-default ${name} ${lib.escapeShellArg value}") settings
        )
      }
    '';
    inherit (repos) meta;
  };
in
{
  options.jitsusama.repos.roots = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    default = [ "${config.home.homeDirectory}/src" ];
    description = ''
      The folders repos looks under for checkouts, four folders deep. A
      tree is found from the checkout it was cut from, wherever it lies,
      so only a folder of trees whose checkout is under no root needs
      naming.
    '';
  };

  options.jitsusama.repos.throughGh = lib.mkOption {
    type = lib.types.listOf (lib.types.strMatching "github:[^/ ]+/[^/ ]+");
    default = [ ];
    example = [ "github:someone/private" ];
    description = ''
      Repositories, by their keys, whose open changes are read through
      gh's own sign-in rather than the GitHub App, which can't see them.
      Read-only and fixed: one query, to GitHub's GraphQL endpoint alone,
      with the token held in memory for each refresh. None by default.
    '';
  };

  config = {
    home.packages = [ wrapped ];

    # The refresh asks only through the broker's socket, so a machine
    # without one skips it rather than failing every five minutes.
    systemd.user.services.repos-refresh = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit = {
        Description = "Asks GitHub after each repository's open changes, for repos";
        After = [ "github-broker.socket" ];
        Wants = [ "github-broker.socket" ];
        ConditionPathExists = "%t/github-broker.sock";
      };
      Service = {
        Type = "oneshot";
        ExecStart = "${lib.getExe wrapped} refresh";
        Slice = "background.slice";
      };
    };
    systemd.user.timers.repos-refresh = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit.Description = "Refreshes repos's GitHub changes every five minutes";
      Timer = {
        OnBootSec = "1min";
        OnUnitActiveSec = "5min";
      };
      Install.WantedBy = [ "timers.target" ];
    };

    # The counting needs only git, so it runs apart from the refresh, which
    # waits on the broker. Its first run counts every head apart from its
    # branch, about a minute on a machine with hundreds of trees; each
    # after counts only the pairs of commits that moved.
    systemd.user.services.repos-count = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit.Description = "Counts how far each head is from its branch, for repos";
      Service = {
        Type = "oneshot";
        ExecStart = "${lib.getExe wrapped} count";
        Slice = "background.slice";
      };
    };
    systemd.user.timers.repos-count = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit.Description = "Counts how far repos's heads are from their branches every five minutes";
      Timer = {
        OnBootSec = "2min";
        OnUnitActiveSec = "5min";
      };
      Install.WantedBy = [ "timers.target" ];
    };
  };
}
