# The checks of the launcher's People, and Tuple's calls with them, which the
# folder's checks.nix gathers with every other place's; its opening comment says
# why each place keeps its own.
{
  account,
  copied,
  graphical,
  lib,
  noFonts,
  pkgs,
  qs,
  shellFolderLint,
  singleton,
  ...
}:
{
  # The launcher's People, with Tuple as its source, reads Tuple's contacts
  # from the real people daemon, which runs the real tuple-people reading
  # a stand-in for Tuple's follow here, and calls, joins or favours one
  # (tests/people.qml), with niri stood in. A listener stands in for
  # Tuple's daemon's socket, and the shell never asks Tuple anything
  # itself, since it reaches Tuple through niri alone.
  the-launcher-finds-the-people =
    let
      niri = singleton "Niri.qml" ''
        property var spawned: []
        function spawn(command) { spawned = command; }
      '';
      report = singleton "Report.qml" "function say(text) {}";
      people = pkgs.callPackage ../../people/package.nix { };
      helper = pkgs.callPackage ../pi-helper.nix {
        name = "tuple-people";
        source = ../../tuple/people.rs;
      };
      contact = id: full: short: email: status: favorited: recent: {
        inherit
          id
          email
          status
          favorited
          recent
          ;
        full_name = full;
        short_name = short;
        kind = "user";
      };
      # Strangers between those Tuple knows, so their order is the search's.
      # Fay comes only once Ada goes into a call, a stranger nobody lists,
      # whom only the daemon's search finds. Gil comes last, met lately,
      # with nobody's stand changed.
      snapshot =
        ada: gil:
        pkgs.writeText "contacts.json" (
          builtins.toJSON (
            [
              (contact 101 "Eli Fenwick" null "eli@example.com" "unknown" false false)
              (contact 102 "Ada Brook" "Ada" "ada@example.com" ada true false)
              (contact 103 null null "gus@example.com" "unknown" false false)
              (contact 104 "Ben Carrow" null "ben@example.com" "busy" true false)
              (contact 105 "Cal Dunmore" null "cal@example.com" "offline" false true)
              (contact 106 "Zoë \"Zed\" Quill" null "zoe@example.com" "unknown" false false)
              (contact 107 "Dee Ellery" null "dee@example.com" "unavailable" true false)
              (contact 108 "" "Hal" null "unknown" false false)
            ]
            ++ lib.optional (ada == "busy") (
              contact 109 "Fay Gale" null "fay@example.com" "unknown" false false
            )
            ++ lib.optional gil (contact 110 "Gil Hart" null "gil@example.com" "unknown" false true)
          )
          + "\n"
        );
      # Tuple's follow says everyone, then again once Ada goes into a call,
      # and once more when Gil comes, and goes on following. Anything else asked of it is written down,
      # since the shell asks Tuple through niri alone.
      tuple = pkgs.writeShellScriptBin "tuple" ''
        if [ "$*" != "--format json contacts follow" ]; then
          echo "$*" >> "$XDG_RUNTIME_DIR/asked"
          exit 0
        fi
        cat ${snapshot "online" false}
        while [ ! -e "$XDG_RUNTIME_DIR/next" ]; do sleep 0.1; done
        cat ${snapshot "busy" false}
        while [ ! -e "$XDG_RUNTIME_DIR/again" ]; do sleep 0.1; done
        cat ${snapshot "busy" true}
        while :; do sleep 1; done
      '';
    in
    pkgs.runCommandLocal "the-launcher-finds-the-people" { } ''
      mkdir -p launcher/people
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      ${copied [
        "Daemon.qml"
        "Words.qml"
        "launcher/Placed.js"
        "launcher/Provider.qml"
        "launcher/ReadyRows.js"
        "launcher/Recent.js"
        "launcher/Zones.js"
        "launcher/words.json"
      ]}
      cp ${../../people/People.qml} launcher/People.qml
      cp ${../../people/Person.js} launcher/Person.js
      cp ${import ../launcher/zones.nix { inherit lib pkgs; }} zones.jsonl
      cp ${../../people/PersonSource.qml} PersonSource.qml
      cp ${../../tuple/TupleSource.qml} launcher/people/tuple.qml
      cp ${../../../tests/people-page-source.qml} launcher/people/pages.qml
      cp ${../../tuple/TuplePeople.qml} TuplePeople.qml
      cp ${../../../tests/people.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${report} Report.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      # Here is UTC, so how far a zone is from here is known.
      export TZ=UTC
      mkdir -m 700 $XDG_RUNTIME_DIR
      mkdir $XDG_RUNTIME_DIR/daemons
      ${lib.getExe pkgs.socat} UNIX-LISTEN:$XDG_RUNTIME_DIR/tuple-0,fork,unlink-close=0 /dev/null &
      while [ ! -S $XDG_RUNTIME_DIR/tuple-0 ]; do sleep 0.1; done
      cat > sources.json <<EOF
      {"sources": [{"name": "tuple", "stands": $(cat ${../../tuple/stands.json}),
        "command": ["${helper}/bin/tuple-people", "people", "${tuple}/bin/tuple", "tuple-0"]},
        {"name": "facts",
          "command": ["${lib.getExe pkgs.bash}", "${../../../tests/people-facts-source.sh}"]}]}
      EOF
      export PEOPLE=${lib.getExe people} SOURCES=$PWD/sources.json
      export WORDS=${(import ../launcher/words.nix).file}
      $PEOPLE $XDG_RUNTIME_DIR/daemons/people.sock $SOURCES $WORDS &
      # The last steps stop the daemon and start it again, to see People
      # say so and then stop saying so.
      echo $! > $XDG_RUNTIME_DIR/people.pid
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      if [ -e $XDG_RUNTIME_DIR/asked ]; then
        echo "the shell asked Tuple itself:"; cat $XDG_RUNTIME_DIR/asked; exit 1
      fi
      touch $out
    '';

  # How the shell says a Tuple call, who's in it and for how long, a
  # call's link in what was copied, and Tuple's Recent, its order and each
  # call's words (tests/tuple.qml, over Tuple.js and the When.js it reads).
  the-desktop-follows-tuple-calls =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-desktop-follows-tuple-calls" { } ''
      cp ${../../../tests/tuple.qml} tuple.qml
      cp ${../../tuple/Tuple.js} Tuple.js
      ${copied [ "When.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml tuple.qml
      touch $out
    '';

  # How the call Joel's in stands comes from tuple-call, the real daemon,
  # over its socket (tests/tuple-call.qml): it follows a stand-in tuple
  # whose stream says each line the test writes, and whose other commands
  # are kept, while a listener stands in for Tuple's daemon answering. The
  # shell tells who comes and goes through a stand-in notify-send that keeps
  # what it's asked. The daemon's own reading of Tuple is its tests', run as
  # it builds (tuple-call-holds-the-call).
  the-desktop-hears-tuples-call =
    let
      calls = pkgs.callPackage ../../tuple/call/package.nix { };
      niri = pkgs.writeText "Niri.qml" ''
        pragma Singleton
        import Quickshell
        import QtQuick
        Singleton {
          property var windows: ({})
          property var casts: ({})
          property var spawned: []
          function spawn(command) { spawned = spawned.concat([command]); }
          function act(request) { }
        }
      '';
      # Tuple's stream says each line of the runtime folder's stream as
      # it's written; `new` opens a room, and every other command is kept.
      tuple = pkgs.writeShellScript "tuple" ''
        if [ "$3 $4" = "state follow" ]; then
          exec ${pkgs.coreutils}/bin/tail -n +1 -f "$XDG_RUNTIME_DIR/stream"
        fi
        echo "$*" >> "$XDG_RUNTIME_DIR/asked"
        if [ "$*" = new ]; then echo "Joining https://tuple.app/c/opened"; fi
      '';
      # Tuple's state as its stream opens: someone signed in, one contact,
      # and Joel's own rooms, the newest of which is his. Tuple names a
      # line's type first, which is all tuple-call reads of a line it
      # skips, so the type isn't left to toJSON's order.
      state = ''{"type":"state","data":${
        builtins.toJSON {
          sign_in = "signed_in";
          contacts = [
            {
              id = 7;
              full_name = "Ada Brightwater";
              email = "ada@example.com";
            }
          ];
          rooms.personal = [
            {
              http_value = "https://tuple.app/c/older";
              created_at = "2025-04-11T12:00:00Z";
            }
            {
              http_value = "https://tuple.app/c/listed";
              created_at = "2026-07-31T12:00:00Z";
            }
          ];
        }
      }}'';
      notify = pkgs.writeShellApplication {
        name = "notify-send";
        text = ''
          printf '%s\n' "$*" >> "$XDG_RUNTIME_DIR/told"
          echo 1
        '';
      };
    in
    pkgs.runCommandLocal "the-desktop-hears-tuples-call" { } ''
      cp ${../../tuple/Tuple.qml} Tuple.qml
      # The copied link Tuple offers to join is read from the shell's one
      # watcher of the clipboard.
      # Tuple's rows are named as the launcher's vocabulary says.
      ${copied [
        "Copied.qml"
        "Daemon.qml"
        "When.js"
        "Words.qml"
        "launcher/words.json"
      ]}
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../../tuple/Tuple.js} Tuple.js
      cp ${../../../tests/tuple-call.qml} shell.qml
      cp ${niri} Niri.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime XDG_DATA_HOME=$TMPDIR/data
      export XDG_STATE_HOME=$TMPDIR/state LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export PATH=${notify}/bin:$PATH
      mkdir -m 700 $XDG_RUNTIME_DIR $XDG_RUNTIME_DIR/daemons
      mkdir -p $XDG_DATA_HOME/tuple/0
      touch $XDG_DATA_HOME/tuple/0/log.txt $XDG_RUNTIME_DIR/told
      printf '%s\n' ${lib.escapeShellArg state} > $XDG_RUNTIME_DIR/stream
      ${lib.getExe pkgs.socat} UNIX-LISTEN:$XDG_RUNTIME_DIR/tuple-0,fork /dev/null &
      # Tuple's daemon answers before tuple-call first looks, which would
      # otherwise look again only five seconds on.
      timeout 10 sh -c 'until [ -S "$0" ]; do sleep 0.05; done' $XDG_RUNTIME_DIR/tuple-0
      ${lib.getExe calls} $XDG_RUNTIME_DIR/daemons/tuple-call.sock ${tuple} &
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';
}
