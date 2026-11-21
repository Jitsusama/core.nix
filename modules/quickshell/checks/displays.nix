# The checks of the launcher's Screens, and the desk light beside them, which
# the folder's checks.nix gathers with every other place's; its opening comment
# says why each place keeps its own.
{
  account,
  copied,
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
  # niri's words for why on the : line; and Screens says how many screens
  # there are, as the windows daemon counts them.
  the-screens-go-off-on-niri = launcherOnNiri {
    name = "the-screens-go-off-on-niri";
    tests = ../../../tests/screens-off-on-niri-test.py;
  };

  # The launcher tells how the desk light is lit and sets it, and the brightness
  # bar hears its changes but not a camera lighting it; with no light, its row
  # stays and says why (modules/quickshell/tests/desk-light.qml), over a
  # stand-in litra that keeps a light's state and says each change on its watch,
  # as the real one does.
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
            # A watch once litra has ended for good fails at once, as one
            # with litra gone would.
            watch)
              [ ! -e "$XDG_RUNTIME_DIR/litra-ended" ] || exit 1
              said
              echo $$ > "$XDG_RUNTIME_DIR/litra-watch"
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
            # litra's watch ending, and every one after it.
            end)
              touch "$XDG_RUNTIME_DIR/litra-ended"
              kill "$(cat "$XDG_RUNTIME_DIR/litra-watch")"
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
      ${copied [
        "DeskLight.qml"
        "Words.qml"
        "launcher/Light.qml"
        "launcher/Order.js"
        "launcher/Power.js"
        "launcher/Provider.qml"
        "launcher/words.json"
      ]}
      cp ${../tests/desk-light.qml} shell.qml
      cp ${niri} Niri.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export PATH=${litra}/bin:$PATH
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The launcher's Screens draws each screen's row as the windows daemon serves it,
  # makes again only the row a change names, offers Windows the screens laid out by
  # what their rows hold, with no record of niri's, asks the daemon a screen's Return
  # rather than niri, says the Brightness as the place's line serves it, and says a
  # share the daemon refused by its code and how many letters with reports off
  # (modules/quickshell/tests/displays.qml, read by displays-heard.sh), with the
  # daemon and niri stood in. A
  # screen's ways, what :setl power and unplugged do with them, and what the place's
  # :set line serves, are the windows daemon's, tested beside it.
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
      ${copied [
        "Daemon.qml"
        "Options.qml"
        "Words.qml"
        "launcher/Catalogue.js"
        "launcher/Displays.qml"
        "launcher/Plate.js"
        "launcher/Provider.qml"
        "launcher/ReadyRows.qml"
        "launcher/words.json"
      ]}
      cp ${../tests/displays.qml} shell.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${niri} Niri.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR $XDG_RUNTIME_DIR/daemons
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      set -o pipefail
      timeout 60 ${qs} -p shell.qml 2>&1 | tee said
      ${pkgs.lib.getExe pkgs.bash} ${../tests/displays-heard.sh} said
      touch $out
    '';
}
