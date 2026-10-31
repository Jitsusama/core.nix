# nix-offload: lends the machine a builder elsewhere while its machine queue
# has builds one could take, and gives it back once idle or at a deadline each day,
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
  builders =
    name: list:
    pkgs.writeText name (lib.concatMapStrings (builder: "${builder.address} ${builder.line}\n") list);
  standing = builders "nix-offload-standing" cfg.standing;
  beside = builders "nix-offload-beside" cfg.beside;
  builder = lib.types.submodule {
    options = {
      address = lib.mkOption {
        type = lib.types.str;
        description = "Where it answers, `host:port`, when it can be reached.";
      };
      line = lib.mkOption {
        type = lib.types.str;
        description = "Its Nix machines line.";
      };
    };
  };
  names = cfg.standing != [ ] || cfg.beside != [ ];
  sendsElsewhere = cfg.nearby != [ ] || cfg.evaluators != [ ];
in
{
  options.jitsusama.nix-offload = {
    package = lib.mkOption {
      type = lib.types.package;
      default = offload;
      readOnly = true;
      description = ''
        nix-offload itself, for a helper to run its `relay` in front of a
        tunnel it holds.
      '';
    };
    helper = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = ''
        The program that makes a builder (`up NAME SECONDS`, printing its Nix
        machines line once ready and running while it's there), gives one back
        (`down NAME`), and names those there (`alive PREFIX`). Without one,
        none is lent, and only the standing builders are named.
      '';
    };
    prefix = lib.mkOption {
      type = lib.types.str;
      default = "nix-offload-";
      description = ''
        What every builder's name starts with. Only those it asked for are ever
        given back, but one of its own keeps others' apart from its.
      '';
    };
    rate = lib.mkOption {
      type = lib.types.strMatching "[0-9]+(\\.[0-9]{1,3})?";
      default = "0";
      description = ''
        A builder's price an hour, in dollars, for the day's line, where the
        helper says none for what it made (`price DOLLARS` before its line).
      '';
    };
    firstStart = lib.mkOption {
      type = time;
      default = "08:00";
      description = "The first time of day a builder is made, so none is made overnight.";
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
    standing = lib.mkOption {
      type = lib.types.listOf builder;
      default = [ ];
      description = ''
        Builders always there, such as other machines nearby, named while they
        answer, so Nix never waits on one that's away. They're never offered to
        every build: only one that names /run/nix-offload/machines-with-standing
        (`--builders @/run/nix-offload/machines-with-standing`) reaches them,
        beside any lent one, so a build of code they mustn't hold never does.
      '';
    };
    beside = lib.mkOption {
      type = lib.types.listOf builder;
      default = [ ];
      description = ''
        Builders always there that may take any of this machine's work, such as
        a second machine for the same work, named to every build beside any
        lent one while they answer, for their big-parallel work alone, as the
        lent one is. Never named with the standing builders. They're also
        named alone, as their own lines, in /run/nix-offload/machines-beside,
        for a build that may go to them and nowhere else
        (`--builders @/run/nix-offload/machines-beside`), which builds here
        while none answers.
      '';
    };
    nearby = lib.mkOption {
      type = lib.types.listOf (
        lib.types.submodule {
          options = {
            address = lib.mkOption {
              type = lib.types.str;
              description = "Where it answers, `host:port`, when it can be reached.";
            };
            host = lib.mkOption {
              type = lib.types.str;
              description = "The host ssh is given to reach it.";
            };
            systems = lib.mkOption {
              type = lib.types.nonEmptyListOf lib.types.str;
              description = "The systems it builds for, such as `aarch64-darwin`.";
            };
            attributes = lib.mkOption {
              type = lib.types.listOf lib.types.str;
              default = [ ];
              description = ''
                Attributes it takes whatever their system, as patterns where `*` stands
                for any run of characters, such as `checks.x86_64-linux.formatting`: those
                that are mostly evaluation, which it does itself, for a system it only
                emulates.
              '';
            };
          };
        }
      );
      default = [ ];
      description = ''
        Machines nearby that build for systems this one can't, such as a Mac at
        home for its own. `nix-offload there` sends each attribute for one of
        their systems to the first that answers, with the flake itself when
        it's a folder here, so the build needs nothing this machine builds.
      '';
    };
    evaluators = lib.mkOption {
      type = lib.types.listOf (
        lib.types.submodule {
          options = {
            address = lib.mkOption {
              type = lib.types.str;
              description = "Where it answers, `host:port`, when it can be reached.";
            };
            host = lib.mkOption {
              type = lib.types.str;
              description = "The host ssh is given to reach it, as the person's own account.";
            };
            owners = lib.mkOption {
              type = lib.types.nonEmptyListOf lib.types.str;
              description = ''
                The GitHub owners whose repositories it takes, so a machine only some work
                may reach is given that work alone.
              '';
            };
          };
        }
      );
      default = [ ];
      description = ''
        Machines that evaluate a change's checks for `nix-offload there
        --changed`, as changed-checks sends itself, when no builder is lent:
        the first that answers runs changed-checks, and what it names is built
        as any `there` build is. Each needs Nix with flakes, git and network
        access to GitHub, and nothing more.
      '';
    };
    timeZone = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "The zone the times are in, when not the machine's own.";
    };
  };

  config = lib.mkIf (cfg.helper != null || names || sendsElsewhere) {
    home.packages = [ offload ];

    xdg.configFile."nix-offload/evaluators" = lib.mkIf (cfg.evaluators != [ ]) {
      text = lib.concatMapStrings (
        machine: "${machine.address} ${machine.host} ${lib.concatStringsSep "," machine.owners}\n"
      ) cfg.evaluators;
    };

    xdg.configFile."nix-offload/nearby" = lib.mkIf (cfg.nearby != [ ]) {
      text = lib.concatMapStrings (
        machine:
        lib.concatStringsSep " " (
          [
            machine.address
            machine.host
            (lib.concatStringsSep "," machine.systems)
          ]
          ++ lib.optional (machine.attributes != [ ]) (lib.concatStringsSep "," machine.attributes)
        )
        + "\n"
      ) cfg.nearby;
    };

    systemd.user.services.nix-offload = lib.mkIf (cfg.helper != null || names) {
      Unit.Description = "Names this machine's builders, lending one while its queue has builds one could take";
      Service = {
        ExecStart = lib.concatStringsSep " " (
          (
            if cfg.helper == null then
              [
                (lib.getExe offload)
                "stand"
              ]
            else
              [
                (lib.getExe offload)
                "serve --helper ${cfg.helper} --prefix ${cfg.prefix} --rate ${cfg.rate}"
                "--first-start ${cfg.firstStart} --last-start ${cfg.lastStart}"
                "--deadline ${cfg.deadline}"
              ]
          )
          ++ lib.optional (cfg.standing != [ ]) "--standing ${standing}"
          ++ lib.optional (cfg.beside != [ ]) "--beside ${beside}"
        );
        # Named here, since glibc without it reads a zone it can't find as
        # UTC, and says nothing.
        Environment = [
          "TZDIR=${pkgs.tzdata}/share/zoneinfo"
        ]
        ++ lib.optional (cfg.timeZone != null) "TZ=${cfg.timeZone}";
        # It's told to end first, so it ends its helper and journals its
        # builder for the next start to take back, then anything left goes.
        # A switch starts nothing until every stop ends, so a stop is short.
        KillMode = "mixed";
        TimeoutStopSec = "10s";
        Restart = "on-failure";
        Slice = "background.slice";
      };
      Install.WantedBy = [ "default.target" ];
    };
  };
}
