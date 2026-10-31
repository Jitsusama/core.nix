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
            signers = {
              ssh =
                if cfg.signers.ssh == [ ] then
                  null
                else
                  pkgs.writeText "update-to-main-signers" (lib.concatLines cfg.signers.ssh);
              openpgp = cfg.signers.openpgp;
            };
            # The running system's path, so once it has switched the newest
            # update-to-main serves.
            program = "/run/current-system/sw/bin/update-to-main";
            tools = {
              git = lib.getExe pkgs.git;
              gpg = lib.getExe' pkgs.gnupg "gpg";
              ssh_keygen = lib.getExe' pkgs.openssh "ssh-keygen";
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
