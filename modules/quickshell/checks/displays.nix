# The checks of the launcher's Screens, and the desk light beside them, which
# the folder's checks.nix gathers with every other place's; its opening comment
# says why each place keeps its own.
{
  account,
  graphical,
  launcherOnNiri,
  noFonts,
  pkgs,
  qs,
  shellFolderLint,
  ...
}:
{
  # Turn Every Screen Off and :set noscreens ask niri to power its monitors
  # off, which keeps each output connected, read on their way to niri; and
  # a scale niri refuses, turned into its ReturnError on the way, says
  # niri's words for why on the warning line.
  the-screens-go-off-on-niri = launcherOnNiri {
    name = "the-screens-go-off-on-niri";
    tests = ../../../tests/screens-off-on-niri-test.py;
  };

  # How a screen stands, and which act its row serves the : line's :setl
  # power and unplugged do, as tests/screens.qml holds launcher/Screens.js
  # to in Qt's own engine; the ways themselves are the windows daemon's.
  the-launcher-turns-each-screen-off =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-turns-each-screen-off" { } ''
      cp ${../../../tests/screens.qml} screens.qml
      cp ${../launcher/Screens.js} Screens.js
      cp ${../launcher/Numbers.js} Numbers.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml screens.qml
      touch $out
    '';

  # A change to a screen niri refuses says on the warning line what was
  # asked, what wasn't done and niri's own words for why, and one to a
  # screen gone says it's unplugged, as tests/screen-answers.qml holds
  # launcher/ScreenAnswers.js to it in Qt's own engine.
  the-launcher-says-what-niri-refuses =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-says-what-niri-refuses" { } ''
      cp ${../../../tests/screen-answers.qml} screen-answers.qml
      cp ${../launcher/ScreenAnswers.js} ScreenAnswers.js
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml screen-answers.qml
      touch $out
    '';

  # The launcher tells how the desk light is lit and sets it, and the
  # brightness bar hears its changes but not a camera lighting it
  # (tests/desk-light.qml), over a stand-in litra that keeps a light's
  # state and says each change on its watch, as the real one does.
  the-launcher-finds-the-desk-light =
    let
      niri = pkgs.writeText "Niri.qml" ''
        pragma Singleton
        import Quickshell
        import QtQuick
        Singleton { function spawn(command) { Quickshell.execDetached(command); } }
      '';
      litra = pkgs.writeShellApplication {
        name = "litra";
        runtimeInputs = [
          pkgs.coreutils
          pkgs.util-linux
        ];
        text = ''
          state=$XDG_RUNTIME_DIR/litra-state
          feed=$XDG_RUNTIME_DIR/litra-feed
          # Each change waits its turn, as litra's own does under a lock on
          # the light's node (modules/litra/litra.rs): the shell sets
          # brightness and warmth at once, and two changes read at the same
          # time would each write back the other's field as it was.
          if [ "$1" != watch ]; then
            exec 9> "$XDG_RUNTIME_DIR/litra-lock"
            flock 9
          fi
          [ -e "$state" ] || echo "true 40 4000" > "$state"
          read -r on percent kelvin < "$state"
          # A light as litra says it: its lumens are a Glow's, 20 to 250
          # across its share, and it names itself as its HID device does.
          said() {
            printf '[{"node":"/dev/hidraw1","on":%s,"percent":%s,"lumens":%s,"kelvin":%s,%s}]\n' \
              "$on" "$percent" "$((20 + 230 * percent / 100))" "$kelvin" \
              '"dimmest":20,"brightest":250,"name":"Litra Glow stand-in","serial":"stand-in"' \
              >> "$feed"
          }
          case "$1" in
            watch)
              said
              exec tail -n +1 -f "$feed"
              ;;
            on) on=true ;;
            off) on=false ;;
            brightness) percent=$2 ;;
            temperature) kelvin=$2 ;;
            # Not litra's: the light's own buttons, or a camera, and the
            # light unplugged.
            press) on=$2 percent=$3 kelvin=$4 ;;
            unplug)
              echo "[]" >> "$feed"
              exit 0
              ;;
            *) exit 2 ;;
          esac
          echo "$on $percent $kelvin" > "$state"
          said
        '';
      };
    in
    pkgs.runCommandLocal "the-launcher-finds-the-desk-light" { } ''
      mkdir launcher
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../Fuzzy.js} Fuzzy.js
      cp ${../DeskLight.qml} DeskLight.qml
      cp ${../launcher/Provider.qml} launcher/Provider.qml
      cp ${../ProgramGlyphs.js} ProgramGlyphs.js
      cp ${../launcher/Asking.js} launcher/Asking.js
      cp ${../launcher/Numbers.js} launcher/Numbers.js
      cp ${../launcher/Light.qml} launcher/Light.qml
      cp ${../launcher/Power.js} launcher/Power.js
      cp ${../launcher/Order.js} launcher/Order.js
      cp ${../Words.qml} Words.qml
      cp ${../launcher/words.json} launcher/words.json
      cp ${../../../tests/desk-light.qml} shell.qml
      cp ${niri} Niri.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export PATH=${litra}/bin:$PATH
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The launcher's Screens draws each screen's row as the windows daemon
  # serves it, makes again only the row a change names, and asks the daemon
  # a screen's Return rather than niri (tests/displays.qml), with the
  # daemon and niri stood in.
  the-launcher-remakes-only-the-screens-that-changed =
    let
      # niri as Screens still asks it, a change to a screen answered.
      niri = ../tests/stand-ins/displays-niri.qml;
    in
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Quickshell runs them with.
    pkgs.runCommandLocal "the-launcher-remakes-only-the-screens-that-changed" { } ''
      mkdir -p launcher
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../Daemon.qml} Daemon.qml
      cp ${../Hearing.js} Hearing.js
      cp ${../Fuzzy.js} Fuzzy.js
      cp ${../ProgramGlyphs.js} ProgramGlyphs.js
      cp ${../launcher/Provider.qml} launcher/Provider.qml
      cp ${../launcher/Asking.js} launcher/Asking.js
      cp ${../launcher/Backlight.js} launcher/Backlight.js
      cp ${../launcher/Catalogue.js} launcher/Catalogue.js
      cp ${../launcher/Displays.qml} launcher/Displays.qml
      cp ${../launcher/Numbers.js} launcher/Numbers.js
      cp ${../launcher/Order.js} launcher/Order.js
      cp ${../launcher/Plate.js} launcher/Plate.js
      cp ${../launcher/Power.js} launcher/Power.js
      cp ${../launcher/ReadyRows.qml} launcher/ReadyRows.qml
      cp ${../launcher/ReadyRows.js} launcher/ReadyRows.js
      cp ${../launcher/Ago.js} launcher/Ago.js
      cp ${../launcher/Interfaces.js} launcher/Interfaces.js
      cp ${../When.js} When.js
      cp ${../launcher/ScreenAnswers.js} launcher/ScreenAnswers.js
      cp ${../launcher/Screens.js} launcher/Screens.js
      cp ${../launcher/Seen.js} launcher/Seen.js
      cp ${../launcher/Warmth.js} launcher/Warmth.js
      cp ${../Words.qml} Words.qml
      cp ${../launcher/words.json} launcher/words.json
      cp ${../../../tests/displays.qml} shell.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${niri} Niri.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR $XDG_RUNTIME_DIR/daemons
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';
}
