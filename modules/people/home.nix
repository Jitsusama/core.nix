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
#
# A source whose program speaks People's line protocol names it as its
# daemon command, and the people daemon (src/main.rs) runs each such
# program, holding everyone they know ready over
# $XDG_RUNTIME_DIR/daemons/people.sock in the shell's daemon interface,
# for the shell and anything else that asks.
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
  # Those the daemon runs, each by its program on People's line protocol.
  daemonSources = lib.filter (source: source.daemon != null) sources;
  people = pkgs.callPackage ./package.nix { };
  sourcesFile = pkgs.writeText "people-sources.json" (
    builtins.toJSON {
      sources = map (source: {
        inherit (source) name;
        command = source.daemon;
      }) daemonSources;
    }
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
          daemon = lib.mkOption {
            type = lib.types.nullOr (lib.types.listOf lib.types.str);
            default = null;
            description = ''
              The program and its arguments the people daemon runs for this
              source, speaking People's line protocol: an address on a line
              in, as a step-in, `?ADDRESS` as an ask ahead, and a JSON line
              out for each, with its `email`; or, first saying
              `{"roster": true}`, a line for everyone it knows unasked, and
              `"gone": true` for one it no longer does. None for a source
              the daemon doesn't run.
            '';
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
      sources = map (
        source:
        removeAttrs source [
          "source"
          "daemon"
        ]
      ) sources;
      # The time where someone is, for a source that knows their time zone,
      # told from the same tzdata list the launcher's clocks read.
      zones = "${import ../quickshell/launcher/zones.nix { inherit lib pkgs; }}";
    };

    # The people daemon, wherever some source names a program for it. It
    # isn't in background.slice: what it holds is what the launcher shows
    # as Joel steps into someone, so it answers ahead of the work behind.
    systemd.user.services.people = lib.mkIf (daemonSources != [ ]) {
      Unit = {
        Description = "Holds everyone Joel deals with ready for the shell";
        # Restarted after a switch reloads the units, it comes back with the
        # sources the switch named, as the new one.
        X-SwitchMethod = "restart";
      };
      Service = {
        ExecStart = "${lib.getExe people} %t/daemons/people.sock ${sourcesFile}";
        Restart = "on-failure";
        RestartSec = 5;
        ExecStopPost = "${pkgs.coreutils}/bin/rm -f %t/daemons/people.sock";
      };
      Install.WantedBy = [ "default.target" ];
    };
  };
}
