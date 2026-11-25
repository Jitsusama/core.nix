# Gives the home folders of the programs Joel waits on, and of the ones that
# write beside them all day, Btrfs subvolumes of their own, moved there as
# the account's manager starts (own-subvolumes.sh says how, and how it
# fails open).
#
# A home is one Btrfs tree, and every write in it takes the lock on that
# tree's root in turn. Probed on 24 November under agents' load, a small
# write and fsync, as SQLite commits one, waited a median of 137 to 981 ms
# a minute in the home's tree, most of it on that lock, and 9 to 27 ms in a
# fresh subvolume beside it, where the lock never showed. Both still took
# the stalls the whole filesystem takes as it commits.
#
# Each module whose folder should be its own adds it to
# jitsusama.subvolumes.folders, with the units that write in it, so the
# folder goes wherever the program does, and the machines that don't
# import it never see it.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.subvolumes;
  ownSubvolumes = pkgs.writeShellApplication {
    name = "own-subvolumes";
    runtimeInputs = [
      pkgs.btrfs-progs
      pkgs.coreutils
      pkgs.findutils
      pkgs.gawk
    ];
    text = builtins.readFile ./own-subvolumes.sh;
  };
  # A unit's name from the folder's path: .config/google-chrome gives
  # own-subvolume-config-google-chrome.
  unitOf =
    folder:
    "own-subvolume-"
    + lib.concatStringsSep "-" (
      lib.filter (part: part != "") (lib.splitString "/" (lib.replaceStrings [ "." ] [ "" ] folder))
    );
  # One unit a folder, so a folder waits only for its own writers and a
  # slow copy holds back nothing but them. It's wanted by the account's
  # manager, never needed by a writer: a run that fails leaves the folder
  # as it was, and they start all the same.
  unitFor = folder: settings: {
    name = unitOf folder;
    value = {
      Unit = {
        Description = "Moves ~/${folder} into a subvolume of its own";
        Before = settings.before;
      };
      Service = {
        Type = "oneshot";
        RemainAfterExit = true;
        ExecStart = "${lib.getExe ownSubvolumes} ${toString settings.seconds} %h/${folder}";
        # The copy is bounded, and the rest is a look at /proc and a
        # rename or two, so this is only a backstop.
        TimeoutStartSec = settings.seconds + 60;
      };
      Install.WantedBy = [ "default.target" ];
    };
  };
in
{
  options.jitsusama.subvolumes.folders = lib.mkOption {
    type = lib.types.attrsOf (
      lib.types.submodule {
        options = {
          before = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            default = [ ];
            description = ''
              The account's units that write in the folder and start with
              its manager, so they wait for the move rather than find it,
              or hold the folder open and keep it where it is. A writer
              the session waits on makes the session wait too.
            '';
          };
          seconds = lib.mkOption {
            type = lib.types.ints.positive;
            default = 300;
            description = ''
              How long the copy may take before it's given up, the folder
              left as it was and tried again at the next login. Under
              agents' load on 25 November, Slack's profile of 53,000 files
              took 165 s, and Chrome's of 12,000, 25 s.
            '';
          };
        };
      }
    );
    default = { };
    description = ''
      Folders under the home, by their paths from it, each moved into a
      Btrfs subvolume of its own at a login that finds nothing holding it.
    '';
  };

  config = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
    systemd.user.services = lib.mapAttrs' unitFor cfg.folders;

    # systemd orders nothing against a unit it doesn't have, so a writer
    # named wrongly would race its folder's move unseen.
    assertions = lib.concatLists (
      lib.mapAttrsToList (
        folder: settings:
        map (writer: {
          assertion = config.systemd.user.services ? ${lib.removeSuffix ".service" writer};
          message = "~/${folder}'s move comes before ${writer}, which isn't a user service here";
        }) settings.before
      ) cfg.folders
    );
  };
}
