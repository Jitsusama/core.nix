# A Mac's name lookups, timed each minute (lookups.sh): each name the
# account depends on, through the system's resolver and each server named,
# and mDNSResponder's share of a core. A lookup that fails or keeps slow, or
# an mDNSResponder that keeps busy, is said as an error, which the unified
# log keeps and health's day counts, so a storm of lookups is seen as it
# starts rather than when an agent can't reach its model. Linux's resolver
# has its own journal, which health reads already.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  settings = config.jitsusama.name-lookups;
  state = "${config.home.homeDirectory}/Library/Caches/name-lookups";
in
{
  options.jitsusama.name-lookups = {
    names = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      # git's, which any account fetches by.
      default = [ "github.com" ];
      description = ''
        The names looked up each minute, through the system's resolver and
        each server in `servers`: those the account can't work without.
      '';
    };
    servers = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      description = ''
        DNS servers each name is also asked of directly, by address, so a
        slow lookup is told apart from a slow server. Every name goes to
        every server, in plain DNS, so name only servers that may see them
        all and can answer them all.
      '';
    };
    slow = lib.mkOption {
      type = lib.types.ints.positive;
      # A lookup takes tens of milliseconds on a well Mac; the storm of 23
      # November took 5 to 17 seconds, and 1.75 s cold after it.
      default = 500;
      description = "How long a lookup may take, in milliseconds, before it counts as slow.";
    };
  };

  config.launchd.agents.name-lookups = lib.mkIf pkgs.stdenv.hostPlatform.isDarwin {
    enable = true;
    config = {
      ProgramArguments = [
        (lib.getExe pkgs.bash)
        "${./lookups.sh}"
        state
        (toString settings.slow)
        (lib.concatStringsSep "," settings.names)
        (lib.concatStringsSep "," settings.servers)
      ];
      # macOS's own dscacheutil, logger and ps, and Nix's timeout and dig.
      EnvironmentVariables.PATH = "${
        lib.makeBinPath [
          pkgs.coreutils
          pkgs.dig
        ]
      }:/usr/bin:/bin:/usr/sbin";
      StartInterval = 60;
      RunAtLoad = true;
      # Standard, not Background, which keeps a job to the efficiency cores
      # at low priority, so a Mac busy building would time its own wait for
      # a core as a slow lookup. A round is a few lookups a minute.
      ProcessType = "Standard";
      # Only what's wrong, and what's well again, a line each.
      StandardErrorPath = "${config.home.homeDirectory}/Library/Logs/name-lookups.log";
    };
  };
}
