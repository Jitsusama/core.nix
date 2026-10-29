# nix-offload: lends the machine a builder elsewhere while its machine queue
# holds builds back, and gives it back once idle or at a deadline each day,
# so none is kept overnight (offload/src/main.rs says how). It decides when;
# the machine's helper says how, so core names no place a builder lives.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.nix-offload;
  offload = pkgs.callPackage ./offload/package.nix { };
  time = lib.types.strMatching "[0-2][0-9]:[0-5][0-9]";
in
{
  options.jitsusama.nix-offload = {
    helper = lib.mkOption {
      type = lib.types.path;
      description = ''
        The program that makes a builder (`up NAME SECONDS`, printing its Nix
        machines line once ready and running while it's there), gives one back
        (`down NAME`), and names those there (`alive PREFIX`).
      '';
    };
    prefix = lib.mkOption {
      type = lib.types.str;
      description = "What every builder's name starts with, so a leftover is known.";
    };
    rate = lib.mkOption {
      type = lib.types.strMatching "[0-9]+(\\.[0-9]{1,3})?";
      description = "A builder's price an hour, in dollars, for the day's line.";
    };
    lastStart = lib.mkOption {
      type = time;
      default = "19:30";
      description = "The last time of day a builder is made, so none lives only minutes.";
    };
    deadline = lib.mkOption {
      type = time;
      default = "20:00";
      description = "The time of day every builder is given back by.";
    };
    timeZone = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "The zone the times are in, when not the machine's own.";
    };
  };

  config = {
    home.packages = [ offload ];

    systemd.user.services.nix-offload = {
      Unit.Description = "Lends this machine a builder while its queue holds builds back";
      Service = {
        ExecStart = lib.concatStringsSep " " [
          (lib.getExe offload)
          "serve --helper ${cfg.helper} --prefix ${cfg.prefix} --rate ${cfg.rate}"
          "--last-start ${cfg.lastStart} --deadline ${cfg.deadline}"
        ];
        # Named here, since glibc without it reads a zone it can't find as
        # UTC, and says nothing.
        Environment = [
          "TZDIR=${pkgs.tzdata}/share/zoneinfo"
        ]
        ++ lib.optional (cfg.timeZone != null) "TZ=${cfg.timeZone}";
        # It's told to end first, so it gives its builder back, then its
        # helper goes.
        KillMode = "mixed";
        TimeoutStopSec = "5min";
        Restart = "on-failure";
        Slice = "background.slice";
      };
      Install.WantedBy = [ "default.target" ];
    };
  };
}
