# elsewhere runs an agent's heavy job on another machine of the realm while
# this one is crowded (src/main.rs says how). Importing it puts it on the
# account's path; a machine repository says where it may send and which
# kinds of job, since which machines a realm holds is its own business:
#
#   jitsusama.elsewhere = {
#     machines = [ "far" ];
#     kinds.model-check = {
#       description = "a TLA+ model check";
#       programs = [ "tla-rs" ];
#       setup = ''PATH="$HOME/.local/bin:$PATH"'';
#     };
#   };
#
# and `elsewhere run model-check -- tla-rs Spec.tla` runs on far while this
# machine's processor or memory is over and far's isn't, and here otherwise.
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
        description = "Whether the job's folder goes with it, or the repository holding it.";
      };
      excludes = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ ];
        description = "What isn't copied, as rsync's --exclude reads it.";
      };
      bringsBack = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ ];
        description = "What's copied back once the job ends, relative to its folder.";
      };
    };
  };
in
{
  options.jitsusama.elsewhere = {
    machines = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      description = "The machines a job may go to, as ssh and mycelium name them, the first with room taken.";
    };
    scratch = lib.mkOption {
      type = lib.types.str;
      default = ".cache/elsewhere";
      description = "The folder under a far machine's home a job's copy goes in.";
    };
    realm = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "The realm mycelium is asked about, or its own default.";
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
      inherit (settings) machines scratch realm;
      kinds = lib.mapAttrs (_: kind: {
        inherit (kind)
          description
          programs
          setup
          copies
          excludes
          ;
        brings_back = kind.bringsBack;
      }) settings.kinds;
    };
  };
}
