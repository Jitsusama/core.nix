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
            belowLoad = mkOption {
              type = types.numbers.positive;
              description = ''
                The minute's load average they build under; at it or above,
                or when it can't be read, the update fails and the machine
                stays as it is.
              '';
            };
          };
        }
      );
      default = null;
      example = {
        belowLoad = 30;
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
                  them in the store at the hashes the locks name.
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
    assertions = [
      {
        assertion = cfg.hosted == null || (cfg.signers.ssh == [ ] && cfg.signers.openpgp == [ ]);
        message = "jitsusama.update-to-main.hosted trusts the host, so it takes no signers.";
      }
      {
        assertion = cfg.kept.names == [ ] || cfg.elsewhere;
        message = "jitsusama.update-to-main.kept needs elsewhere, or kept pieces build here.";
      }
      {
        assertion = (cfg.builders == null && cfg.here == null) || cfg.elsewhere;
        message = "jitsusama.update-to-main.builders and here are for building elsewhere.";
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
                  below_load = cfg.here.belowLoad;
                };
            refuse_while = cfg.refuseWhile;
            # The running system's path, so once it has switched the newest
            # update-to-main serves.
            program = "/run/current-system/sw/bin/update-to-main";
            tools = {
              git = lib.getExe pkgs.git;
              gpg = lib.getExe' pkgs.gnupg "gpg";
              ssh_keygen = lib.getExe' pkgs.openssh "ssh-keygen";
              tar = lib.getExe pkgs.gnutar;
            }
            // cfg.system.tools;
          }
          // removeAttrs cfg.system [ "tools" ]
        )
      );
    };

    environment.systemPackages = [ cfg.package ];
    environment.etc."update-to-main/config.json".source = cfg.configFile;
  };
}
