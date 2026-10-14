# network-revive gives the account's units another go when the network
# changes (src/main.rs says how). systemd's user manager knows nothing of
# the network, so a unit that needs it to start fails on a boot whose
# network comes late, and stays failed once its restarts run out; and one
# that looks its names up only as it starts keeps what it found on the
# network before. A module names such units here:
#
#   jitsusama.network-revive.units."proxy.service" = {
#     check = [ "-t" ];
#     reload = true;
#   };
#
# With none named, nothing runs.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.network-revive;
  package = pkgs.callPackage ./package.nix { };
  configFile = pkgs.writeText "network-revive.json" (
    builtins.toJSON {
      units = lib.mapAttrsToList (name: unit: {
        inherit name;
        inherit (unit) check reload;
      }) cfg.units;
    }
  );
in
{
  options.jitsusama.network-revive.units = lib.mkOption {
    description = "The account's services the network can let down, by their unit's name.";
    default = { };
    type = lib.types.attrsOf (
      lib.types.submodule {
        options = {
          check = lib.mkOption {
            description = ''
              The arguments that make the service's own program say whether
              it could start, added to its ExecStart, as nginx's -t. It's
              started or reloaded only once that passes, so nothing starts
              only to fail. With none, it's started unchecked.
            '';
            type = lib.types.nullOr (lib.types.listOf lib.types.str);
            default = null;
          };
          reload = lib.mkOption {
            description = ''
              Whether it's reloaded when the network changes while it runs,
              for a program that looks its names up only as it reads its
              configuration.
            '';
            type = lib.types.bool;
            default = false;
          };
        };
      }
    );
  };

  config = lib.mkIf (cfg.units != { }) {
    assertions = [
      {
        assertion = lib.all (lib.hasSuffix ".service") (lib.attrNames cfg.units);
        message = "network-revive starts and checks services only, named with .service.";
      }
    ];

    systemd.user.services.network-revive = {
      Unit.Description = "Gives units the network let down another go";
      Service = {
        ExecStart = "${lib.getExe package} ${configFile}";
        # Whatever the account runs comes first; this only waits.
        Slice = "background.slice";
        # What it finds each time the network changes is said at debug
        # level, for a drop-in to let through when it's needed.
        LogLevelMax = "info";
        Restart = "on-failure";
        RestartSec = "5s";
      };
      Install.WantedBy = [ "default.target" ];
    };
  };
}
