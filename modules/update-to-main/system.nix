# What update-to-main is told on either system: the repository and branch
# the machine follows, the system it builds from them, whose signatures
# count, who may ask, and how to tell the machine is whole after a switch.
# nixos.nix and darwin.nix add what differs between them and run it as
# root. src/lib.rs says what it does with an ask.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.update-to-main;
  inherit (lib) mkOption types;

  # Whose signatures on a main's head let it be built: the machine's own
  # repository's, or a followed input's.
  signers = {
    ssh = mkOption {
      type = types.listOf types.singleLineStr;
      default = [ ];
      description = ''
        SSH signers whose signature on main's head lets it be built, as
        lines of git's allowed-signers file: a principal, then the key.
      '';
      example = [ ''someone namespaces="git" ssh-ed25519 AAAA...'' ];
    };
    openpgp = mkOption {
      type = types.listOf types.path;
      default = [ ];
      description = ''
        OpenPGP public keys, each an armoured file, whose signature on
        main's head lets it be built, such as the key a forge signs its
        merges with.
      '';
    };
  };
  signersFor = given: {
    ssh =
      if given.ssh == [ ] then
        null
      else
        pkgs.writeText "update-to-main-signers" (lib.concatLines given.ssh);
    inherit (given) openpgp;
  };
in
{
  options.jitsusama.update-to-main = {
    repository = mkOption {
      type = types.str;
      description = "The repository the machine follows, as git fetches it.";
      example = "https://github.com/someone/machines";
    };
    branch = mkOption {
      type = types.str;
      default = "main";
      description = "The branch whose head the machine runs.";
    };
    output = mkOption {
      type = types.str;
      description = "The flake attribute of the machine's system in that repository.";
      example = "nixosConfigurations.laptop.config.system.build.toplevel";
    };
    inherit signers;
    follows = mkOption {
      type = types.attrsOf (
        types.submodule {
          options = {
            repository = mkOption {
              type = types.str;
              description = "The input's own repository, as git fetches it.";
            };
            branch = mkOption {
              type = types.str;
              default = "main";
              description = "The branch whose head the input follows.";
            };
            inherit signers;
          };
        }
      );
      default = { };
      description = ''
        The flake's inputs, by their names in it, that follow their own
        repository's main rather than where the flake's lock pins them:
        each built at its main once one of its signers signed it and it
        descends from the lock's commit and from what the machine last
        followed. Main counts as moved when any of them moved.
      '';
    };
    elsewhere = mkOption {
      type = types.bool;
      default = false;
      description = ''
        Whether the machine builds only the light pieces of its system
        itself, those nixpkgs marks to build where they're asked for,
        leaving every other to `builders` and failing rather than building
        one here when none takes it, unless `here` lets it.
      '';
    };
    builders = mkOption {
      type = types.nullOr types.str;
      default = null;
      example = "@/run/builders-for-anything";
      description = ''
        The builders the pieces built elsewhere go to, as Nix's `builders`
        setting takes them; none for the Nix daemon's own. A builder takes
        only pieces that ask for what it must have, so one kept for big
        builds leaves the rest to `here`.
      '';
    };
    here = mkOption {
      type = types.nullOr (
        types.submodule {
          options = {
            maxJobs = mkOption {
              type = types.ints.positive;
              default = 1;
              description = "How many pieces build here at once.";
            };
            cores = mkOption {
              type = types.ints.positive;
              default = 4;
              description = "How many cores each may use.";
            };
            busyAt = mkOption {
              type = types.numbers.positive;
              default = 15;
              description = ''
                The share of the last minute, in percent, that some runnable
                task waited for a CPU, at or above which they wait: the
                update fails and the machine stays as it is. Where that
                can't be read, as on a Mac, they always wait.
              '';
            };
            belowLoad = mkOption {
              type = types.nullOr types.numbers.positive;
              default = null;
              description = ''
                Read no longer: `busyAt` gates on CPU pressure instead of the
                load average, which with many sessions open stayed over any
                useful bound. Taken away once no machine sets it.
              '';
            };
          };
        }
      );
      default = null;
      example = {
        busyAt = 15;
      };
      description = ''
        When set, pieces built elsewhere that their builders don't build,
        because none takes them or none answers, are built here instead,
        small and only while the machine is quiet, rather than the update
        failing. Kept pieces built here go to no builder at all; the rest
        still go to theirs when they can.
      '';
    };
    kept = {
      names = mkOption {
        type = types.listOf types.str;
        default = [ ];
        example = [ "secret-tool*" ];
        description = ''
          Pieces of the system, built elsewhere, that go only to `builders`
          and never to the Nix daemon's own, by patterns over each piece's
          derivation name or its `pname`, where `*` stands for any run of
          characters. Every piece built from one is kept too, since it
          would carry that one to whichever builder took it.
        '';
      };
      builders = mkOption {
        type = types.nullOr types.str;
        default = null;
        example = "@/run/kept-builders";
        description = ''
          The builders the kept pieces go to, as Nix's `builders` setting
          takes them. None builds none of them, so an update needing a kept
          piece fails and the machine stays as it is.
        '';
      };
    };
    batches = mkOption {
      type = types.nullOr (
        types.submodule {
          options = {
            pieces = mkOption {
              type = types.ints.positive;
              default = 100;
              description = "How many pieces one build is given at most.";
            };
            roomMiB = mkOption {
              type = types.ints.positive;
              default = 6144;
              description = ''
                How much memory available and swap free together, in MiB,
                Linux has to say there is before a batch, or the build
                after them, starts.
              '';
            };
            within = mkOption {
              type = types.ints.positive;
              default = 1800;
              description = ''
                How many seconds an update waits for that room before it
                fails, leaving the machine as it is.
              '';
            };
          };
        }
      );
      default = null;
      example = { };
      description = ''
        When set, a system built here, on NixOS, that needs more pieces
        built than a batch holds is built a batch at a time, the pieces
        built from none of the others first, each batch once the machine
        has the room for it. The Nix daemon's worker serving one build
        grows with what it's given, so a small machine's one big build can
        run out of memory where its batches don't.
      '';
    };
    refuseWhile = mkOption {
      type = types.listOf (
        types.submodule {
          options = {
            command = mkOption {
              type = types.nonEmptyListOf types.str;
              description = ''
                Run as root, it succeeds while the machine is busy and exits 1
                while it isn't, within 30 seconds. Anything else counts as
                busy, since it couldn't tell.
              '';
            };
            why = mkOption {
              type = types.str;
              example = "a call is on";
              description = "What the machine is busy with, as the refusal says it.";
            };
            followerWaits = mkOption {
              type = types.bool;
              default = false;
              description = ''
                Whether it refuses nothing, and only a switch the follower
                asked for waits on it once built, for an hour at most, such
                as someone at the machine: that's no reason to refuse an ask
                they made themselves, nor to build nothing ahead.
              '';
            };
            followerWaitsAtMost = mkOption {
              type = types.nullOr types.ints.positive;
              default = null;
              example = 2700;
              description = ''
                How many seconds at most a switch the follower asked for
                waits on it, after which it holds the switch no longer,
                though what else the follower waits on still does. With
                none, it waits the whole hour. A switch a person asked for
                waits on it as long, when every ask waits on it.
              '';
            };
            everyAskWaits = mkOption {
              type = types.bool;
              default = false;
              description = ''
                Whether a switch a person asked for waits on it too, as one
                the follower asked for does, when the follower waits on it,
                such as the launcher being typed into: a person's ask is no
                reason to restart what someone is typing into.
              '';
            };
          };
        }
      );
      default = [ ];
      description = ''
        What the machine may be busy with when an update mustn't start or
        switch, such as a call a switch would cut. Each is asked before the
        update starts and again before it switches; while one is busy the
        ask is refused, and what was built stays for the next one.
      '';
    };
    holdAccountRestarts = mkOption {
      type = types.attrsOf (types.listOf types.str);
      default = { };
      example = {
        joel = [ "sessions-kitty.service" ];
      };
      description = ''
        Units of an account's own systemd, by the account, that a switch
        mustn't restart through home-manager until an asker grants it, as
        holdRestarts' daemons wait, such as one a person's windows run
        under. A unit whose new file says `X-RestartIfChanged=false` isn't
        restarted, so it needs no grant. NixOS only.
      '';
    };
    holdChanges = mkOption {
      type = types.listOf types.str;
      default = [ ];
      example = [ "kernel" ];
      description = ''
        Paths in the system whose changing waits for an asker's grant, as
        holdRestarts' daemons do, such as `kernel` on NixOS for a machine
        whose kernel should change only with someone there.
      '';
    };
    holdRestarts = mkOption {
      type = types.listOf types.str;
      default = [ ];
      example = [ "nix-daemon.service" ];
      description = ''
        Daemons a switch mustn't restart until an asker grants it, having
        held the machine's other work first: systemd units on NixOS. A
        switch that would restart one is refused, what was built stays for
        the next ask, and an asker grants the next switch by making
        `/run/update-to-main-hold/given`, which that switch takes away.
      '';
    };
    hosted = mkOption {
      type = types.nullOr (
        types.submodule (
          { config, ... }:
          {
            options = {
              user = mkOption {
                type = types.str;
                description = ''
                  The account whose logins reach the host and the flake's
                  private inputs, and whose checkout holds the repository's
                  objects.
                '';
              };
              checkout = mkOption {
                type = types.str;
                description = ''
                  That account's checkout of the repository. Root reads its
                  objects, which git checks against the commit the host
                  names, and none of its configuration.
                '';
              };
              directory = mkOption {
                type = types.str;
                default = "";
                description = "The folder in the repository the flake is in, empty for its top.";
              };
              token = mkOption {
                type = types.nonEmptyListOf types.str;
                description = ''
                  A command, run as the account in its checkout, that prints a
                  short-lived token the host takes as a bearer.
                '';
              };
              changed = mkOption {
                type = types.nullOr (types.nonEmptyListOf types.str);
                default = null;
                description = ''
                  A command, run as the account in its checkout, that prints
                  the newest commit on main that changed the folder. When
                  main's head isn't in the checkout but that commit is, and
                  its folder is the one last built, the ask fetches and
                  builds nothing. The account's word only spares a fetch:
                  what's built is always a commit the host named. When it
                  fails, or names a commit the checkout lacks, main is
                  fetched as it would be without it.
                '';
              };
              api = mkOption {
                type = types.nullOr types.str;
                default = null;
                example = "https://github.example.com/api/v3/repos/owner/repo";
                description = ''
                  The host's REST API for the repository, which root asks
                  with the token for the folder's trees and blobs at main's
                  head, each checked against the id the tree above it
                  names, so main needn't be fetched into the checkout: a
                  fetch that moves a large repository's main checks the
                  new head's whole tree is there, which took minutes. Only
                  the commit's tree, and whether main descends from what
                  the machine runs, are the host's word, as main's head
                  is. When the API fails, main is fetched.
                '';
              };
              mayFetch = mkOption {
                type = types.nullOr (types.nonEmptyListOf types.str);
                default = null;
                example = [
                  "sh"
                  "-c"
                  "h=$(date +%H); [ \"$h\" -ge 2 ] && [ \"$h\" -lt 7 ]"
                ];
                description = ''
                  A command, run as the account in its checkout, that
                  succeeds when main may be fetched into the checkout now:
                  a fetch that moves a large repository's main can take
                  minutes and gigabytes. When it fails, a head the checkout
                  lacks and the API couldn't read is left for a later ask,
                  and the machine takes `fetched`'s commit or stays where it
                  is, with only its followed inputs moved on. When null,
                  main is fetched whenever it's needed.
                '';
              };
              fetched = mkOption {
                type = types.nullOr (types.nonEmptyListOf types.str);
                default = null;
                example = [
                  "git"
                  "rev-parse"
                  "refs/prefetch/remotes/origin/main"
                ];
                description = ''
                  A command, run as the account in its checkout, that
                  prints a commit of main the checkout already holds, such
                  as a ref another fetch keeps. While main mayn't be
                  fetched, the machine takes it when the host says main
                  holds it and it descends from what the machine last
                  switched to, so the account's word can at most keep the
                  machine behind main's head. It's taken only while the
                  host's API answers: on a day the API is down, the machine
                  stays where it is and only what it follows moves on.
                '';
              };
              path = mkOption {
                type = types.listOf types.str;
                default = [
                  "/etc/profiles/per-user/${config.user}/bin"
                  "/run/current-system/sw/bin"
                  "/usr/bin"
                  "/bin"
                ];
                description = "The folders the account's commands find their programs in.";
              };
              inputs = mkOption {
                type = types.listOf types.str;
                default = [ ];
                description = ''
                  The flake's inputs only the account can fetch, which it
                  fetches by their locks before each build, so root finds
                  them in the store at the hashes the locks name. On a Mac
                  they're fetched in the account's login session while one
                  is open, since the keychain its logins are kept in answers
                  nothing else.
                '';
              };
            };
          }
        )
      );
      default = null;
      description = ''
        When set, main's head is the commit the repository's host names,
        asked over TLS with the account's token, rather than one a signature
        vouches for: for a host whose merges aren't signed. The machine then
        trusts the host, and takes no signers.
      '';
    };
    askers = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = ''
        The accounts that may ask for an update, through the socket only
        their group writes. An agent asks as one of them, through the node.
      '';
    };
    follower = mkOption {
      type = types.nullOr types.str;
      default = null;
      example = "someone";
      description = ''
        The account, one of the askers, whose service (a launchd agent on
        a Mac) asks for an update whenever main or a followed input moves,
        and at least every ten minutes; none to ask only when someone does.
      '';
    };
    followerTells = mkOption {
      type = types.nullOr (types.nonEmptyListOf types.str);
      default = null;
      description = ''
        The command the follower runs, as its account, to tell its person
        the machine can't update: once for heads that fail, and for a
        refusal only a person's ask gets past, never for being busy. Why
        is given after its own arguments. None to only log it.
      '';
    };
    health = mkOption {
      type = types.listOf (types.nonEmptyListOf types.str);
      description = ''
        Commands that each have to succeed after a live switch, or the
        machine goes back to the system it left. The Nix daemon answering
        is always one.
      '';
    };
    healthWithin = mkOption {
      type = types.ints.positive;
      default = 120;
      description = "How many seconds the health commands have to succeed.";
    };
    batteryFloor = mkOption {
      type = types.ints.between 0 100;
      default = 20;
      description = "Below this charge, on battery, an update isn't started.";
    };

    # What nixos.nix and darwin.nix fill in, and what they read back.
    system = mkOption {
      type = types.attrs;
      internal = true;
      description = "The config's fields particular to the machine's system.";
    };
    package = mkOption {
      type = types.package;
      readOnly = true;
      internal = true;
      default = pkgs.callPackage ./package.nix { };
    };
    configFile = mkOption {
      type = types.path;
      readOnly = true;
      internal = true;
      description = "The config update-to-main serves by.";
    };
  };

  config = {
    warnings = lib.optional (cfg.here != null && cfg.here.belowLoad != null) ''
      jitsusama.update-to-main.here.belowLoad is read no longer; here.busyAt
      gates on CPU pressure instead, so drop it.
    '';
    assertions = [
      {
        assertion = cfg.follower == null || lib.elem cfg.follower cfg.askers;
        message = "jitsusama.update-to-main.follower has to be one of its askers.";
      }
      {
        assertion = cfg.hosted == null || (cfg.signers.ssh == [ ] && cfg.signers.openpgp == [ ]);
        message = "jitsusama.update-to-main.hosted trusts the host, so it takes no signers.";
      }
      {
        assertion = cfg.kept.names == [ ] || cfg.elsewhere;
        message = "jitsusama.update-to-main.kept needs elsewhere, or kept pieces build here.";
      }
      {
        assertion =
          cfg.holdRestarts ++ cfg.holdChanges == [ ] && cfg.holdAccountRestarts == { }
          || cfg.system ? hold_given;
        message = "jitsusama.update-to-main's holds are for NixOS, where an asker can grant one.";
      }
      {
        assertion = (cfg.builders == null && cfg.here == null) || cfg.elsewhere;
        message = "jitsusama.update-to-main.builders and here are for building elsewhere.";
      }
      {
        assertion = cfg.batches == null || (!cfg.elsewhere && cfg.system.kind == "nixos");
        message = "jitsusama.update-to-main.batches are for building here, on NixOS.";
      }
    ]
    ++ lib.mapAttrsToList (name: follow: {
      assertion = follow.signers.ssh != [ ] || follow.signers.openpgp != [ ];
      message = "jitsusama.update-to-main.follows.${name} needs a signer, or nothing it follows is built.";
    }) cfg.follows;
    jitsusama.update-to-main = {
      health = lib.mkBefore [
        [
          cfg.system.tools.nix
          "--extra-experimental-features"
          "nix-command"
          "store"
          "info"
          # The daemon itself, which root would otherwise go round.
          "--store"
          "daemon"
        ]
      ];
      configFile = pkgs.writeText "update-to-main.json" (
        builtins.toJSON (
          {
            inherit (cfg)
              repository
              branch
              output
              health
              ;
            health_within = cfg.healthWithin;
            battery_floor = cfg.batteryFloor;
            group = "update-ask";
            signers = signersFor cfg.signers;
            follows = lib.mapAttrs (_: follow: {
              inherit (follow) repository branch;
              signers = signersFor follow.signers;
            }) cfg.follows;
            inherit (cfg) hosted elsewhere builders;
            kept = { inherit (cfg.kept) names builders; };
            here =
              if cfg.here == null then
                null
              else
                {
                  max_jobs = cfg.here.maxJobs;
                  inherit (cfg.here) cores;
                  busy_at = cfg.here.busyAt;
                };
            batches =
              if cfg.batches == null then
                null
              else
                {
                  inherit (cfg.batches) pieces within;
                  room_mib = cfg.batches.roomMiB;
                };
            refuse_while = map (busy: {
              inherit (busy) command why;
              follower_waits = busy.followerWaits;
              follower_waits_at_most = busy.followerWaitsAtMost;
              every_ask_waits = busy.everyAskWaits;
            }) cfg.refuseWhile;
            follower_tells = cfg.followerTells;
            holds =
              if cfg.holdRestarts ++ cfg.holdChanges == [ ] && cfg.holdAccountRestarts == { } then
                null
              else
                {
                  units = cfg.holdRestarts;
                  paths = cfg.holdChanges;
                  account_units = cfg.holdAccountRestarts;
                  given = cfg.system.hold_given;
                };
            # The running system's path, so once it has switched the newest
            # update-to-main serves.
            program = "/run/current-system/sw/bin/update-to-main";
            tools = {
              git = lib.getExe pkgs.git;
              gpg = lib.getExe' pkgs.gnupg "gpg";
              ssh_keygen = lib.getExe' pkgs.openssh "ssh-keygen";
              tar = lib.getExe pkgs.gnutar;
              curl = lib.getExe pkgs.curl;
            }
            // cfg.system.tools;
          }
          // removeAttrs cfg.system [
            "tools"
            "hold_given"
          ]
        )
      );
    };

    environment.systemPackages = [ cfg.package ];
    environment.etc."update-to-main/config.json".source = cfg.configFile;
  };
}
