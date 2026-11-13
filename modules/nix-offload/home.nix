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
  builders = lib.concatMapStrings (builder: "${builder.address} ${builder.line}\n");
  # The lender reads these again as it names its builders, so a switch
  # that changes who they are changes no unit, and is taken up at once
  # rather than once the lent builder goes idle.
  listed = name: "${config.xdg.configHome}/nix-offload/${name}";
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
      free = lib.mkOption {
        type = lib.types.nullOr lib.types.path;
        default = null;
        example = lib.literalExpression ''
          pkgs.writeShellScript "free-work" "exec ssh -i ~/.ssh/collect builder@work collect 20"
        '';
        description = ''
          For a builder beside, a program this account runs to free its
          store while its proofs find it full, under the floor, as the
          linux-builder's `collectableBy` lets a key do. It runs at once,
          then at most every 15 minutes while it stays full, and is ended
          past 10. Unset, a full one waits to collect its own.
        '';
      };
    };
  };
  names = cfg.standing != [ ] || cfg.beside != [ ];
  # How a lender is run, the first and any for VM tests alike.
  serving = [
    (lib.getExe offload)
    "serve --helper ${cfg.helper} --rate ${cfg.rate}"
    "--first-start ${cfg.firstStart} --last-start ${cfg.lastStart}"
    "--deadline ${cfg.deadline}"
    "--outlasting ${outlasting}"
  ]
  ++ lib.optional (cfg.longest != null) "--longest ${cfg.longest}";
  # What a helper holding a builder runs through: a scope of its own, which
  # the unit's end leaves, so a lender that restarts after a switch hands
  # its builder over and the builds on its way finish. Waiting for none to
  # use it instead, lenders ran the old binary for hours under load on
  # 8 November, and restarting them by hand cut a switch's build.
  outlasting = pkgs.writeShellScript "nix-offload-outlasting" ''
    exec ${lib.getExe' pkgs.systemd "systemd-run"} --user --scope --quiet --collect \
      --slice=${stopping.Slice} -- "$@"
  '';
  # What warms a VM builder for core's own VM tests: the driver of a desktop
  # one, whose machine nearly every VM run since 6 November shares,
  # run-subtests' included, built from core's main as the builder stands.
  warmOfCore = [
    "github:Jitsusama/core.nix#checks.x86_64-linux.vm-desktop-launcher-finds-by-name.driver"
  ];
  # What a VM lender has each builder it stands build before it takes VM
  # tests, so none waits an hour on a cold one.
  warming = lib.optional (
    cfg.vmTests != null && cfg.vmTests.warm != [ ]
  ) "--warm ${lib.escapeShellArg (lib.concatStringsSep " " cfg.vmTests.warm)}";
  # How long a VM lender keeps a builder nothing uses, since one given
  # back costs its successor a boot and a warming.
  staying = lib.optional (cfg.vmTests != null) "--idle ${cfg.vmTests.idle}";
  # Named here, since glibc without it reads a zone it can't find as UTC,
  # and says nothing.
  zoned = [
    "TZDIR=${pkgs.tzdata}/share/zoneinfo"
  ]
  ++ lib.optional (cfg.timeZone != null) "TZ=${cfg.timeZone}";
  # It's told to end first, so it ends its helper and journals its builder
  # for the next start to take back, then anything left goes. A switch
  # starts nothing until every stop ends, so a stop is short.
  stopping = {
    KillMode = "mixed";
    TimeoutStopSec = "10s";
    Restart = "on-failure";
    Slice = "offload.slice";
    # A switch that changes it reloads it rather than restarting it: its
    # end ends the way to its builder, and every build reaching it with
    # that, which cut 8.1 hours of builds at three switches on 3 November.
    # Told so, it ends at once, exiting 100, which systemd counts as a
    # success to start the new unit after; a builder a build reaches is
    # handed over, its way outlasting the unit (`outlasting`).
    # It waits only while a builder it handed to a successor finishes.
    ExecReload = "${lib.getExe' pkgs.coreutils "kill"} -HUP $MAINPID";
    SuccessExitStatus = "100";
    RestartForceExitStatus = "100";
    # Its builders are reached with keys of their own, never the account's
    # agent, which ssh would ask first: on 8 November an agent waiting on
    # its person stalled the helpers' ssh at login, and every builder was
    # lost.
    UnsetEnvironment = "SSH_AUTH_SOCK";
  };
  # Where home-manager's sd-switch reads that a changed unit is reloaded,
  # in [Unit], for the reason `stopping` gives.
  reloaded = {
    X-ReloadIfChanged = true;
  };
  # What a VM-test lender's helper is told: its own environment, and how
  # many VM tests its builder runs at once as OFFLOAD_MAX_JOBS, from the one
  # number `there` splits them by, so the two can't differ.
  vmEnvironment =
    environment:
    zoned
    ++ lib.mapAttrsToList (name: value: "${name}=${value}") (
      environment
      // lib.optionalAttrs (cfg.vmTests.builds != null) {
        OFFLOAD_MAX_JOBS = toString cfg.vmTests.builds;
      }
    );
  # How many VM lenders past the second there may be, one for each folder
  # nixos.nix makes, /run/nix-offload-vm-3 to -16, and `there` looks in:
  # on 9 November eight were full for twenty minutes, and VM runs waited a
  # median of 7 minutes for them. One lent past those busy costs nothing,
  # since it's lent only while they're full and goes once idle.
  extraVmLenders = 14;
  # The lender of the VM builder numbered `number`, from 3, while
  # vmTests.extra has one for it.
  extraVm =
    number:
    let
      lender = lib.elemAt cfg.vmTests.extra (number - 3);
      lends = cfg.vmTests != null && lib.length cfg.vmTests.extra > number - 3;
    in
    lib.mkIf (cfg.helper != null && lends) {
      Unit = {
        Description = "Lends another builder for VM tests while those before it are full";
      }
      // reloaded;
      Service = {
        ExecStart = lib.concatStringsSep " " (
          serving
          ++ [
            "--prefix ${lender.prefix}"
            "--dir /run/nix-offload-vm-${toString number} --only-there"
          ]
          ++ warming
          ++ staying
        );
        Environment = vmEnvironment lender.environment;
      }
      // stopping;
      Install.WantedBy = [ "default.target" ];
    };
  sendsElsewhere = cfg.nearby != [ ] || cfg.evaluators != [ ];
  # Core's own VM tests, named so by its checks, and run-subtests' build of
  # some of one's subtests, which went the lent way unnamed: up to 2505 s
  # beside kernel builds on 3 November.
  vmTestsOfCore = [
    "checks.*.vm-*"
    "legacyPackages.*.subtests"
  ];
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
        (`down NAME`), and names those there (`alive PREFIX`). Before the line
        it may print `price DOLLARS`, `store URI`, and where the builder is,
        `instance NAME ZONE` and `cloud-token TOKEN`, a token it prints again
        while it holds the builder; each lender writes the last two in
        its folder for the service that checks the line's host key against
        the cloud's. Without one, none is lent, and only the standing builders
        are named.
      '';
    };
    whenNoneAnswers = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = ''
        A program the lender runs the moment no builder answers any work: the
        one lent failed two proofs in a row and nothing stands in. Its first
        argument is `none`, its second a sentence saying why; once a builder
        answers again, or none is wanted, it runs once more with `back` and who.
        It's waited on, so it must be quick, such as one writing a file for
        whoever keeps the machine's builds moving. Without one, the lender
        only says so in its journal.
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
    longest = lib.mkOption {
      type = lib.types.nullOr time;
      default = null;
      example = "12:00";
      description = ''
        The longest each builder lives from its asking, in hours and minutes,
        in place of the deadline, so one lent at any hour isn't ended
        mid-build at midnight. Idle time still gives it back first.
      '';
    };
    vmTests = lib.mkOption {
      type = lib.types.nullOr (
        lib.types.submodule {
          options = {
            prefix = lib.mkOption {
              type = lib.types.str;
              description = "What every VM-test builder's name starts with, apart from the first's.";
            };
            patterns = lib.mkOption {
              type = lib.types.listOf lib.types.str;
              default = [ ];
              description = ''
                The attributes that are VM tests, as patterns where `*` stands
                for any run of characters, beside core's own: its checks named
                `vm-*` and run-subtests' `legacyPackages.*.subtests`. A VM
                test given as its derivation is known by what it needs, kvm
                or nixos-test, whatever its name.
              '';
            };
            environment = lib.mkOption {
              type = lib.types.attrsOf lib.types.str;
              default = { };
              description = ''
                What the helper is told for this builder, such as the port its
                way to it takes, so it holds one beside the first builder's.
              '';
            };
            warm = lib.mkOption {
              type = lib.types.listOf lib.types.str;
              default = warmOfCore;
              description = ''
                What each VM builder builds for itself as it stands, before it
                takes VM tests, as installables its Nix is given. While a warm
                one stands, `there` sends VM tests only to warm ones: on 6
                November a fresh one's first tests took 65 to 80 minutes
                building the desktop, against 2 to 12 once warm. Empty, each
                takes VM tests cold.
              '';
            };
            idle = lib.mkOption {
              type = lib.types.str;
              default = "00:45";
              example = "00:15";
              description = ''
                How long, as HH:MM, each VM builder may sit unused before it's
                given back, where other builders go after 15 minutes. A fresh
                one boots and then warms for about 17 minutes before it takes
                a VM test: on 10 November four given back after 15 minutes
                were asked for again two to four minutes later, and VM runs
                waited a median 6.8 minutes for a builder while they warmed.
              '';
            };
            builds = lib.mkOption {
              type = lib.types.nullOr lib.types.ints.positive;
              default = null;
              example = 6;
              description = ''
                How many VM tests each VM builder runs at once, told to both
                VM lenders' helpers as OFFLOAD_MAX_JOBS, so `there` sends
                what's past that to the second, `more`, rather than leave it
                waiting its turn. Past both, or with no second, a run's VM
                tests wait here for room. Unset, `there` counts 4.
              '';
            };
            more = lib.mkOption {
              type = lib.types.nullOr (
                lib.types.submodule {
                  options = {
                    prefix = lib.mkOption {
                      type = lib.types.str;
                      description = ''
                        What every such builder's name starts with, apart from
                        the others', none of whose prefixes it may start with.
                      '';
                    };
                    environment = lib.mkOption {
                      type = lib.types.attrsOf lib.types.str;
                      default = { };
                      description = ''
                        What the helper is told for this builder, such as the
                        port its way to it takes, so it holds one beside the
                        others'.
                      '';
                    };
                  };
                }
              );
              default = null;
              description = ''
                A second builder for VM tests, lent by the same helper only
                while a `nix-offload there` build waits on it, which takes
                the VM tests past the first's `builds`, counting those every
                run here has sent it, so none waits its turn behind them. It's
                given back idle once the first has room.
              '';
            };
            extra = lib.mkOption {
              type = lib.types.listOf (
                lib.types.submodule {
                  options = {
                    prefix = lib.mkOption {
                      type = lib.types.str;
                      description = ''
                        What every such builder's name starts with, apart from
                        the others', none of whose prefixes it may start with.
                      '';
                    };
                    environment = lib.mkOption {
                      type = lib.types.attrsOf lib.types.str;
                      default = { };
                      description = ''
                        What the helper is told for this builder, such as the
                        port its way to it takes and the host its ssh entry
                        names, so it holds one beside the others'.
                      '';
                    };
                  };
                }
              );
              default = [ ];
              description = ''
                VM builders past the second, at most ${toString extraVmLenders},
                the first of them lent from /run/nix-offload-vm-3, each
                taking what's past the room of those before it while a
                `nix-offload there` build waits on one: on 5 November VM
                runs waited a median of 23 to 27 minutes on two.
              '';
            };
          };
        }
      );
      default = null;
      description = ''
        A second builder, lent by the same helper only while a `nix-offload
        there` build waits on it, that takes the VM tests such a build names,
        so they never share a builder with big builds. Without it, they go to
        the one lent.
      '';
    };
    more = lib.mkOption {
      type = lib.types.nullOr (
        lib.types.submodule {
          options = {
            prefix = lib.mkOption {
              type = lib.types.str;
              description = "What every such builder's name starts with, apart from the others'.";
            };
            environment = lib.mkOption {
              type = lib.types.attrsOf lib.types.str;
              default = { };
              description = ''
                What the helper is told for this builder, such as the port its
                way to it takes, so it holds one beside the others'.
              '';
            };
          };
        }
      );
      default = null;
      description = ''
        Another builder beside the one lent, lent by the same helper only while
        a `nix-offload there` build waits on it, which such a build is sent to
        while the first's CPUs are full, so it takes only what overflows and
        is given back idle once the first has room. Without it, every such
        build goes to the one lent.
      '';
    };
    standing = lib.mkOption {
      type = lib.types.listOf builder;
      default = [ ];
      description = ''
        Builders always there, such as other machines nearby, named while they
        answer, so Nix never waits on one that's away. They're never offered to
        every build: only one that names /run/nix-offload/machines-with-standing
        (`--builders @/run/nix-offload/machines-with-standing`), or, where the
        machine opens one, goes through the door to them
        (`--store unix:///run/nix-offload-standing.sock`), reaches them,
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
        while none answers. The lent one alone, with nothing mandatory, is
        in /run/nix-offload/machines-for-anything, for a build that may send
        any public piece away, such as one with no local jobs; work only
        some machines may take must ask for a feature its line lacks.
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
        One that answers but can't fetch a flake that isn't a folder, as
        when its resolver is down, builds none of it: its share goes the
        lent way with the rest.
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

    xdg.configFile."nix-offload/standing" = lib.mkIf names { text = builders cfg.standing; };

    xdg.configFile."nix-offload/beside" = lib.mkIf names { text = builders cfg.beside; };

    xdg.configFile."nix-offload/freeing" = lib.mkIf names {
      text = lib.concatMapStrings (builder: "${builder.address} ${builder.free}\n") (
        lib.filter (builder: builder.free != null) cfg.beside
      );
    };

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

    # The lenders, their tunnels and relays carry every build sent away, so
    # they weigh as much as a session does rather than background's 30: on
    # 8 November, at 48% CPU pressure, a tunnel in background was so slow to
    # answer that its relay hung up on 11 callers in 9 minutes. Their CPU is a few
    # percent, so the weight buys them time when they need it and little
    # else. The disk stays background's, since an upload's reads can wait.
    systemd.user.slices.offload.Slice = {
      CPUWeight = 100;
      IOWeight = 1;
    };

    systemd.user.services.nix-offload = lib.mkIf (cfg.helper != null || names) {
      Unit = {
        Description = "Names this machine's builders, lending one while its queue has builds one could take";
      }
      // reloaded;
      Service = {
        ExecStart = lib.concatStringsSep " " (
          (
            if cfg.helper == null then
              [
                (lib.getExe offload)
                "stand"
              ]
            else
              serving
              ++ [ "--prefix ${cfg.prefix}" ]
              ++ lib.optional (cfg.whenNoneAnswers != null) "--tell ${cfg.whenNoneAnswers}"
          )
          ++ lib.optionals names [
            "--standing ${listed "standing"}"
            "--beside ${listed "beside"}"
            "--freeing ${listed "freeing"}"
          ]
        );
        Environment = zoned;
      }
      // stopping;
      Install.WantedBy = [ "default.target" ];
    };

    assertions = [
      {
        assertion = cfg.vmTests == null || cfg.vmTests.more == null || cfg.vmTests.builds != null;
        message = ''
          jitsusama.nix-offload.vmTests.more takes the VM tests past the first's
          room, so vmTests.builds must say how many it runs at once.
        '';
      }
      {
        assertion =
          cfg.vmTests == null
          || cfg.vmTests.extra == [ ]
          || (cfg.vmTests.more != null && lib.length cfg.vmTests.extra <= extraVmLenders);
        message = ''
          jitsusama.nix-offload.vmTests.extra takes what's past the second VM
          builder, so vmTests.more must be set, and it holds at most
          ${toString extraVmLenders}, as many folders as nixos.nix makes for them.
        '';
      }
    ];

    xdg.configFile."nix-offload/vm-tests" = lib.mkIf (cfg.vmTests != null) {
      text = lib.concatMapStrings (pattern: "${pattern}\n") (vmTestsOfCore ++ cfg.vmTests.patterns);
    };

    xdg.configFile."nix-offload/vm-builds" = lib.mkIf (cfg.vmTests.builds or null != null) {
      text = "${toString cfg.vmTests.builds}\n";
    };

    systemd.user.services.nix-offload-vm = lib.mkIf (cfg.helper != null && cfg.vmTests != null) {
      Unit = {
        Description = "Lends a builder for VM tests alone while a build sent there waits on one";
      }
      // reloaded;
      Service = {
        ExecStart = lib.concatStringsSep " " (
          serving
          ++ [
            "--prefix ${cfg.vmTests.prefix}"
            "--dir /run/nix-offload-vm --only-there"
          ]
          ++ warming
          ++ staying
        );
        Environment = vmEnvironment cfg.vmTests.environment;
      }
      // stopping;
      Install.WantedBy = [ "default.target" ];
    };

    systemd.user.services.nix-offload-vm-more =
      lib.mkIf (cfg.helper != null && cfg.vmTests.more or null != null)
        {
          Unit = {
            Description = "Lends a second builder for VM tests while the first is full";
          }
          // reloaded;
          Service = {
            ExecStart = lib.concatStringsSep " " (
              serving
              ++ [
                "--prefix ${cfg.vmTests.more.prefix}"
                "--dir /run/nix-offload-vm-more --only-there"
              ]
              ++ warming
              ++ staying
            );
            Environment = vmEnvironment cfg.vmTests.more.environment;
          }
          // stopping;
          Install.WantedBy = [ "default.target" ];
        };

    systemd.user.services.nix-offload-vm-3 = extraVm 3;
    systemd.user.services.nix-offload-vm-4 = extraVm 4;
    systemd.user.services.nix-offload-vm-5 = extraVm 5;
    systemd.user.services.nix-offload-vm-6 = extraVm 6;
    systemd.user.services.nix-offload-vm-7 = extraVm 7;
    systemd.user.services.nix-offload-vm-8 = extraVm 8;
    systemd.user.services.nix-offload-vm-9 = extraVm 9;
    systemd.user.services.nix-offload-vm-10 = extraVm 10;
    systemd.user.services.nix-offload-vm-11 = extraVm 11;
    systemd.user.services.nix-offload-vm-12 = extraVm 12;
    systemd.user.services.nix-offload-vm-13 = extraVm 13;
    systemd.user.services.nix-offload-vm-14 = extraVm 14;
    systemd.user.services.nix-offload-vm-15 = extraVm 15;
    systemd.user.services.nix-offload-vm-16 = extraVm 16;

    systemd.user.services.nix-offload-more = lib.mkIf (cfg.helper != null && cfg.more != null) {
      Unit = {
        Description = "Lends another builder while the first is full and a build sent there waits on one";
      }
      // reloaded;
      Service = {
        ExecStart = lib.concatStringsSep " " (
          serving
          ++ [
            "--prefix ${cfg.more.prefix}"
            "--dir /run/nix-offload-more --only-there"
          ]
        );
        Environment = zoned ++ lib.mapAttrsToList (name: value: "${name}=${value}") cfg.more.environment;
      }
      // stopping;
      Install.WantedBy = [ "default.target" ];
    };
  };
}
