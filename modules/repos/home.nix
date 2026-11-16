# repos lists every repository on the machine once, with every checkout of
# it and every tree cut from each (src/lib.rs says how). It reads git's own
# files as it runs, and merges in what GitHub last held, which a timer asks
# every five minutes through the GitHub broker (src/github.rs says why).
# Where it looks is the one setting that differs between machines: a
# machine whose trees lie in a folder of their own, under no checkout,
# adds that folder.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  repos = pkgs.callPackage ./package.nix { };
  # The command with its roots set, as a default a caller may still set
  # for one run.
  wrapped = pkgs.symlinkJoin {
    name = "repos-${repos.version}";
    paths = [ repos ];
    nativeBuildInputs = [ pkgs.makeWrapper ];
    postBuild = ''
      wrapProgram $out/bin/repos \
        --set-default REPOS_ROOTS ${lib.escapeShellArg (lib.concatStringsSep ":" config.jitsusama.repos.roots)}
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
  };
}
