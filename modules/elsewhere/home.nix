# elsewhere runs an agent's heavy job on whichever machine of the realm
# mycelium places it (src/main.rs says how). Importing it puts it on the
# account's path; a machine repository says where it may send and which
# kinds of job, since which machines a realm holds is its own business:
#
#   jitsusama.elsewhere = {
#     machines = [ "far" ];
#     kinds.model-check = {
#       description = "a TLA+ model check";
#       programs = [ "tla-rs" ];
#       setup = ''PATH="$HOME/.local/bin:$PATH"'';
#       lasts = "30m";
#     };
#   };
#
# and `elsewhere run model-check -- tla-rs Spec.tla` runs on far when
# `mycelium place` puts the work there, and here otherwise.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  settings = config.jitsusama.elsewhere;

  kind = lib.types.submodule {
    options = {
      description = lib.mkOption {
        type = lib.types.str;
        default = "";
        description = "What a job of this kind is, for `elsewhere kinds`.";
      };
      programs = lib.mkOption {
        type = lib.types.nonEmptyListOf lib.types.str;
        description = ''
          The programs a job of this kind may start, by name. On the far
          machine each is found on its own path, since a path here names a
          build for this machine.
        '';
      };
      setup = lib.mkOption {
        type = lib.types.lines;
        default = "";
        description = "A shell line the far machine runs before the job, such as one setting its path.";
      };
      copies = lib.mkOption {
        type = lib.types.enum [
          "folder"
          "repository"
        ];
        default = "folder";
        description = "Whether the tracked files of the job's folder go with it, or those of the repository holding it. Hidden files, keys and certificates never do.";
      };
      excludes = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ ];
        description = "What of the tracked files doesn't travel: a path from the tree's top when it starts with /, else a name at any depth. A tracked file under a folder named target or node_modules travels, so name here any build output a repository tracks.";
      };
      bringsBack = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ ];
        description = "What's copied back once the job ends, relative to its folder.";
      };
      needs = lib.mkOption {
        type = lib.types.listOf (lib.types.strMatching "[a-z]+=.+");
        default = [ ];
        example = [ "system=darwin" ];
        description = "What a machine must be to take a job of this kind, as `mycelium place --need` reads it.";
      };
      lasts = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        example = "30m";
        description = "How long its jobs run, as `mycelium place --lasts` reads it, so none goes to a machine leaving sooner.";
      };
    };
  };
in
{
  options.jitsusama.elsewhere = {
    machines = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      description = "The machines a job may go to, as mycelium names them; one mycelium places it on elsewhere stays here.";
    };
    scratch = lib.mkOption {
      # One hidden name, as elsewhere checks it, so a bad one fails the build
      # rather than every elsewhere command after it.
      type = lib.types.strMatching "[.]([^./][^/]*|[.][^/]+)" // {
        description = "one hidden folder's name, such as .elsewhere-runs";
      };
      default = ".elsewhere-runs";
      description = "The one folder within the quest's scratch on a far machine that a job's copy runs in, taken away once it ends; a strict fence there writes nowhere else. Hidden, so no share carries a copy home, and so never the handover.";
    };
    realm = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "The realm mycelium is asked about, or its own default.";
    };
    ledgers = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      # Where mycelium's home.nix has the realm's node keep its state: its
      # own default for the home realm, and a folder of the realm's name
      # for any other it also joins.
      default =
        let
          home = config.jitsusama.mycelium.realm or null;
          apart = settings.realm != null && settings.realm != home;
        in
        "${config.xdg.stateHome}/mycelium${lib.optionalString apart "/${settings.realm}"}/shared";
      defaultText = lib.literalExpression ''"''${config.xdg.stateHome}/mycelium/shared"'';
      description = "The folder where the realm's node keeps a ledger of each shared quest, read for the machines already holding a job's quest, which its placement then prefers, since a job sent there waits for fewer files to cross; null reads none.";
    };
    kinds = lib.mkOption {
      type = lib.types.attrsOf kind;
      default = { };
      description = "The kinds of job that may go elsewhere, by name.";
    };
  };

  config = {
    home.packages = [ (pkgs.callPackage ./package.nix { }) ];
    xdg.configFile."elsewhere/settings.json".text = builtins.toJSON {
      inherit (settings)
        machines
        scratch
        realm
        ledgers
        ;
      kinds = lib.mapAttrs (_: kind: {
        inherit (kind)
          description
          programs
          setup
          copies
          excludes
          needs
          lasts
          ;
        brings_back = kind.bringsBack;
      }) settings.kinds;
    };
  };
}
