# Keeps core's lock on the inputs named following their upstreams by
# itself: every 15 minutes, lock-follows (lock-follows.sh) opens a draft
# pull request moving each to its upstream's newest commit, once main's
# lock is behind, and closes the one it replaces. It never merges, so each
# still lands through review and the checks a lock change runs. Cadence's
# mycelium-to-lock measure bars the wait at an hour, and a bump by hand
# waits on someone noticing.
#
# It works in a clone of its own under the account's state folder, and
# runs git, gh and nix as the account does, with the account's settings,
# so its commits are signed as the account's are and its pull requests
# are the account's; the machine gives nix. Each run adds a JSON line to
# runs.jsonl beside the clone.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.lock-follows;
  program = pkgs.callPackage ./package.nix { };
  state = "${config.xdg.stateHome}/lock-follows";
  # The account's own tools come first, the machine's next, and gh last,
  # for an account without its own.
  path = lib.concatStringsSep ":" [
    "/etc/profiles/per-user/${config.home.username}/bin"
    "${config.home.homeDirectory}/.nix-profile/bin"
    "/run/wrappers/bin"
    "/run/current-system/sw/bin"
    (lib.makeBinPath [ pkgs.gh ])
  ];
in
{
  options.jitsusama.lock-follows = {
    checkout = lib.mkOption {
      description = ''
        The clone lock-follows works in, its own, made from `repository`
        on its first run. Anything left in it is thrown away each run.
      '';
      type = lib.types.str;
      default = "${state}/core.nix";
      defaultText = lib.literalExpression ''"''${config.xdg.stateHome}/lock-follows/core.nix"'';
    };
    repository = lib.mkOption {
      description = "Where the clone comes from and its pull requests go.";
      type = lib.types.str;
      default = "https://github.com/Jitsusama/core.nix";
    };
    inputs = lib.mkOption {
      description = ''
        The flake inputs to follow, each by its name in flake.nix. Every
        other input is left as main locks it.
      '';
      type = lib.types.listOf lib.types.str;
      default = [ "mycelium" ];
    };
    branchPrefix = lib.mkOption {
      description = ''
        The start of each branch it pushes, as PREFIX/INPUT-REV8, which is
        how it knows its own pull requests.
      '';
      type = lib.types.strMatching "[A-Za-z0-9._-]+(/[A-Za-z0-9._-]+)*";
      default = "lock-follows";
    };
    prCommand = lib.mkOption {
      description = ''
        The command pull requests are listed, opened and closed with,
        taking gh's words: gh from the account's PATH unless named.
      '';
      type = lib.types.str;
      default = "gh";
    };
  };

  config = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
    home.packages = [ program ];

    systemd.user.services.lock-follows = {
      Unit.Description = "Opens a pull request moving core's lock to each followed input's newest";
      Service = {
        Type = "oneshot";
        ExecStart = lib.escapeShellArgs (
          [
            (lib.getExe program)
            "--checkout"
            cfg.checkout
            "--repository"
            cfg.repository
            "--prefix"
            cfg.branchPrefix
            "--record"
            "${state}/runs.jsonl"
            "--pr"
            cfg.prCommand
            "--"
          ]
          ++ cfg.inputs
        );
        Environment = [ "PATH=${path}" ];
        # Whatever the account runs comes first.
        Slice = "background.slice";
      };
    };
    systemd.user.timers.lock-follows = {
      Unit.Description = "Follows core's locked inputs to their upstreams every 15 minutes";
      Timer = {
        OnCalendar = "*:0/15";
        # A quarter slept through runs once the machine wakes.
        Persistent = true;
      };
      Install.WantedBy = [ "timers.target" ];
    };
  };
}
