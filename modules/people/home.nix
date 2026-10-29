# People: everyone Joel deals with, one person per email address however
# many programs know them, in the launcher's People place. Each program
# that knows people is a source: a QML file starting from PersonSource.qml,
# named in jitsusama.people.sources by the module that brings the program,
# which says who it knows, what's recent with them, drawn by that thing's
# own row, and what Joel can do with them through it. People.qml joins
# them by address (Person.js) and never names a source, so a source plugs
# in from anywhere, a machine's own repository included, with no change
# here. A module with a source imports this one, so People is there only
# where some program knows people.
{ quickshell }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  # The sources in the order they're consulted, lowest first, which is the
  # order their acts are listed in and whose name for someone wins.
  sources = lib.sortOn (source: source.order) (
    lib.mapAttrsToList (name: source: source // { inherit name; }) config.jitsusama.people.sources
  );
in
{
  imports = [ quickshell ];

  options.jitsusama.people.sources = lib.mkOption {
    type = lib.types.attrsOf (
      lib.types.submodule {
        # A source's own settings sit beside these and go to its file as
        # they are.
        freeformType = lib.types.attrsOf lib.types.anything;
        options = {
          order = lib.mkOption {
            type = lib.types.int;
            description = ''
              Where it's consulted, lowest first: its acts are listed in this
              order, and the first real name a source gives someone is theirs.
            '';
          };
          source = lib.mkOption {
            type = lib.types.path;
            description = "What knows them: a QML file starting from PersonSource.qml.";
          };
        };
      }
    );
    default = { };
    description = ''
      The programs that know people, by name, each with its order and its
      source, and settings of its own beside them.
    '';
  };

  config = {
    xdg.configFile = {
      "quickshell/PersonSource.qml".source = ./PersonSource.qml;
      "quickshell/launcher/Person.js".source = ./Person.js;
    }
    // lib.listToAttrs (
      map (
        source:
        lib.nameValuePair "quickshell/launcher/people/${source.name}.qml" {
          inherit (source) source;
        }
      ) sources
    );

    # Just before Calls, since calling someone is the first thing done with
    # a person.
    jitsusama.quickshell.launcher.providers.people = {
      order = 44;
      source = ./People.qml;
      weight = 0.9;
      sources = map (source: removeAttrs source [ "source" ]) sources;
      # The time where someone is, for a source that knows their time zone,
      # told from the same tzdata list the launcher's clocks read.
      zones = "${import ../quickshell/launcher/zones.nix { inherit lib pkgs; }}";
    };
  };
}
